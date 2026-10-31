//! Learned level-neutral operation for the two coloration stages.
//!
//! The static trim tables in [`super::output_stage`] remove the average gain offset of each
//! amplifier / transformer pairing. What is left is programme- and level-dependent, so the per
//! module AUTO switch used to chase it with a slow RMS rider. That rider, by construction, moves
//! gain with the music. This module replaces the chase with a one-shot measurement:
//!
//! * Three shadow [`OutputStage`]s run side by side while the user plays material. Shadow `i`
//!   processes the same input at PUSH state `i` (0 = subtle, 1 = normal, 2 = smashed) with the
//!   static tables engaged and the slow rider disabled.
//! * For each state the input / output power ratio is the residual the tables missed. It is
//!   frozen into a per-state trim, so all three PUSH states become level-neutral from a single
//!   pass.
//! * Once frozen the values never move (a latch, not a chase).
//!
//! Learning is always explicit: the user holds AUTO. A plain click only engages or disengages the
//! stage, and re-engaging reuses the last frozen result. Results are stored per sensor key
//! (amplifier model + transformer model + sample rate), so switching models and back never forces
//! a new measurement, and a double-click can drop the stored result for the current key.
//!
//! The measurement is gated on programme level so silence cannot teach anything, needs at least
//! half a second of signal, and gives up after a generous timeout (both measured in samples so
//! the result is identical at any buffer size).

use super::output_stage::OutputStage;
use super::shared::StereoFrame;

/// A larger residual than this means something is wrong with the calibration, not with the
/// programme. Clamping keeps a bad measurement from becoming a gain swing.
const MAX_TRIM_DB: f64 = 6.0;
/// Ignore samples quieter than this (~ -60 dBFS per sample).
const GATE_POW: f64 = 1.0e-6;
/// Measurement length when the debug menu does not override it.
const DEFAULT_LEARN_SECONDS: f64 = 12.0;
/// Ignore the first fraction of a second while the filter and hysteresis states settle.
const SETTLE_SECONDS: f64 = 0.25;
/// Below this much signal a measurement is not trustworthy.
const MIN_SECONDS: f64 = 2.0;
/// Give up on this stretch of programme and start listening again.
const TIMEOUT_SECONDS: f64 = 45.0;
/// Shortest measurement the debug override may ask for. Below this the settle window would eat
/// most of the pass.
const MIN_LEARN_SECONDS: f64 = 0.5;
/// Longest measurement the debug override may ask for.
const MAX_LEARN_SECONDS: f64 = 180.0;

/// First float of a persisted table. Older builds stored a single raw snapshot whose first slot
/// was the amplifier index, so this value can never collide with one of those blobs.
const BLOB_MAGIC: f32 = 7319.0;
const BLOB_HEADER: usize = 2;
const ENTRY_LEN: usize = 9;
/// One slot per amplifier / transformer / sample-rate combination seen this session.
pub const MAX_LEARNED: usize = 64;

/// A frozen measurement for one sensor key.
#[derive(Clone, Copy, Debug, Default)]
struct LearnedTrim {
    amp: i32,
    trans: i32,
    sample_rate: f32,
    trims_db: [f32; 3],
    pre_gain_db: [f32; 3],
}

impl LearnedTrim {
    fn matches(&self, amp: i32, trans: i32, sample_rate: f32) -> bool {
        self.amp == amp && self.trans == trans && (self.sample_rate - sample_rate).abs() < 1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LearnState {
    /// Nothing measured for this key yet.
    Idle,
    /// Collecting programme.
    Learning,
    /// Measurement frozen and in use.
    Latched,
}

/// One-shot AUTO measurement for a single coloration stage.
pub struct ColorationAuto {
    amp: i32,
    trans: i32,
    sample_rate: f32,
    configured: bool,
    enabled: bool,
    shadows: [OutputStage; 3],
    state: LearnState,
    trims_db: [f32; 3],
    pre_gain_db: [f32; 3],
    /// Frozen results for every key measured this session (and restored from the project state).
    learned: [Option<LearnedTrim>; MAX_LEARNED],
    /// Debug override for the measurement length; `0` means [`DEFAULT_LEARN_SECONDS`].
    learn_seconds: f64,
    in_pow: f64,
    out_pow: [f64; 3],
    settle: usize,
    count: usize,
    elapsed: usize,
    target: usize,
    min_samples: usize,
    timeout: usize,
    dirty: bool,
}

impl Default for ColorationAuto {
    fn default() -> Self {
        let mut shadows = [
            OutputStage::default(),
            OutputStage::default(),
            OutputStage::default(),
        ];
        for shadow in &mut shadows {
            // The shadows must reproduce exactly what the static tables do, and nothing else.
            shadow.set_compensation(true);
            shadow.set_auto_level_enabled(false);
            shadow.set_residual_trim_db(0.0);
            shadow.set_pre_gain_db(0.0);
        }
        Self {
            amp: -1,
            trans: -1,
            sample_rate: 0.0,
            configured: false,
            enabled: false,
            shadows,
            state: LearnState::Idle,
            trims_db: [0.0; 3],
            pre_gain_db: [0.0; 3],
            learned: [None; MAX_LEARNED],
            learn_seconds: 0.0,
            in_pow: 0.0,
            out_pow: [0.0; 3],
            settle: 0,
            count: 0,
            elapsed: 0,
            target: 1,
            min_samples: 1,
            timeout: 1,
            dirty: false,
        }
    }
}

impl ColorationAuto {
    /// True when the frozen result on hand was measured for this exact sensor.
    pub fn key_matches(&self, amp: i32, trans: i32, sample_rate: f32) -> bool {
        self.configured
            && self.amp == amp
            && self.trans == trans
            && (self.sample_rate - sample_rate).abs() < 1.0
    }

    /// Point the engine at a (possibly new) amplifier / transformer / sample rate.
    ///
    /// When the key is unchanged this is a no-op. When it changed, the frozen result for the new
    /// key is restored from the in-memory table, or adopted from the persisted blob on the first
    /// visit. Nothing is measured here: a model switch never starts a learn pass.
    ///
    /// The blob is only read on an actual key change, so the audio thread never locks in the hot
    /// path.
    pub fn configure(&mut self, amp: i32, trans: i32, sample_rate: f32, saved: &[f32]) {
        if self.key_matches(amp, trans, sample_rate) {
            return;
        }
        self.amp = amp;
        self.trans = trans;
        self.sample_rate = sample_rate;
        self.configured = true;
        for shadow in &mut self.shadows {
            shadow.set_sample_rate(sample_rate);
        }
        if self.lookup(amp, trans, sample_rate).is_none() {
            self.load_blob(saved);
        }
        self.adopt_stored();
        self.reset_accumulators();
        self.dirty = false;
    }

    /// Engage or disengage the stage. This never starts a measurement: learning is hold-only.
    pub fn set_enabled(&mut self, on: bool) {
        if self.enabled == on {
            return;
        }
        self.enabled = on;
        if !on && self.state == LearnState::Learning {
            // Abandon the pass and fall back to the last frozen result, so a disengage followed by
            // a re-engage returns to exactly the state the user left.
            self.abort_learn();
        }
    }

    /// Debug override for the measurement length. `0` restores the default.
    ///
    /// Applying it while a pass is running retargets that pass, so the debug menu takes effect
    /// immediately instead of only on the next hold.
    pub fn set_learn_seconds(&mut self, seconds: f64) {
        if (self.learn_seconds - seconds).abs() < f64::EPSILON {
            return;
        }
        self.learn_seconds = seconds;
        if self.state == LearnState::Learning {
            self.retarget();
        }
    }

    /// User held AUTO: measure this sensor again. The previous result stays in the table until the
    /// new pass finishes, so an aborted pass cannot leave the stage without a value.
    pub fn relearn(&mut self) {
        self.enabled = true;
        self.begin_learn();
    }

    /// Double-click on AUTO: forget the frozen result for the current sensor only.
    pub fn clear_learned(&mut self) {
        let (amp, trans, sample_rate) = (self.amp, self.trans, self.sample_rate);
        self.remove(amp, trans, sample_rate);
        self.trims_db = [0.0; 3];
        self.pre_gain_db = [0.0; 3];
        self.reset_accumulators();
        self.state = LearnState::Idle;
        self.dirty = true;
    }

    pub fn reset_dsp(&mut self) {
        for shadow in &mut self.shadows {
            shadow.reset();
        }
    }

    /// 0.0 = idle / disabled, 0..1 = measuring, 1.0 = frozen and in use.
    pub fn progress(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        match self.state {
            LearnState::Latched => 1.0,
            LearnState::Learning => {
                (0.01 + 0.98 * (self.count as f32 / self.target.max(1) as f32)).min(0.99)
            }
            LearnState::Idle => 0.0,
        }
    }

    /// Trim to apply for the current PUSH state, or `None` while the values are not frozen.
    pub fn latched_trim_db(&self, intensity: i32) -> Option<f64> {
        if self.enabled && self.state == LearnState::Latched {
            Some(self.trims_db[intensity.clamp(0, 2) as usize] as f64)
        } else {
            None
        }
    }

    /// Run the three shadows over one input frame and accumulate the next measurement.
    ///
    /// Cheap no-op unless a measurement is in progress.
    pub fn observe(&mut self, input: StereoFrame) {
        if !self.enabled || self.state != LearnState::Learning {
            return;
        }
        let in_inst = 0.5
            * ((input.left as f64) * (input.left as f64)
                + (input.right as f64) * (input.right as f64));
        let mut out_inst = [0.0_f64; 3];
        for (i, shadow) in self.shadows.iter_mut().enumerate() {
            let out = shadow.process_with_params(input, self.amp, self.trans, i as i32);
            out_inst[i] = 0.5
                * ((out.left as f64) * (out.left as f64) + (out.right as f64) * (out.right as f64));
        }

        if self.settle > 0 {
            self.settle -= 1;
            return;
        }
        if in_inst > GATE_POW {
            self.in_pow += in_inst;
            for (dst, src) in self.out_pow.iter_mut().zip(out_inst.iter()) {
                *dst += *src;
            }
            self.count += 1;
        }
        self.elapsed += 1;

        if self.count >= self.target || self.elapsed >= self.timeout {
            if self.count >= self.min_samples {
                self.finish_learn();
            } else {
                // Not enough programme to trust: start listening again from a clean slate.
                self.reset_accumulators();
            }
        }
    }

    /// True exactly once after the table changes; the caller then saves the snapshot.
    pub fn take_dirty(&mut self) -> bool {
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    /// Serialise every frozen result so the project restores them without re-measuring.
    ///
    /// Layout: `[magic, count, (amp, trans, sample_rate, trim[0..3], pre_gain[0..3]) * count]`.
    pub fn snapshot_blob(&self) -> Vec<f32> {
        let count = self.learned.iter().flatten().count();
        let mut blob = Vec::with_capacity(BLOB_HEADER + count * ENTRY_LEN);
        blob.push(BLOB_MAGIC);
        blob.push(count as f32);
        for entry in self.learned.iter().flatten() {
            blob.extend_from_slice(&[
                entry.amp as f32,
                entry.trans as f32,
                entry.sample_rate,
                entry.trims_db[0],
                entry.trims_db[1],
                entry.trims_db[2],
                entry.pre_gain_db[0],
                entry.pre_gain_db[1],
                entry.pre_gain_db[2],
            ]);
        }
        blob
    }

    fn effective_learn_seconds(&self) -> f64 {
        if self.learn_seconds > 0.0 {
            self.learn_seconds
                .clamp(MIN_LEARN_SECONDS, MAX_LEARN_SECONDS)
        } else {
            DEFAULT_LEARN_SECONDS
        }
    }

    fn lookup(&self, amp: i32, trans: i32, sample_rate: f32) -> Option<LearnedTrim> {
        self.learned
            .iter()
            .flatten()
            .copied()
            .find(|entry| entry.matches(amp, trans, sample_rate))
    }

    fn store(&mut self, entry: LearnedTrim) {
        if let Some(slot) = self
            .learned
            .iter_mut()
            .find(|slot| slot.is_some_and(|e| e.matches(entry.amp, entry.trans, entry.sample_rate)))
        {
            *slot = Some(entry);
            return;
        }
        if let Some(slot) = self.learned.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(entry);
            return;
        }
        // Table full: the oldest-looking slot is as good as any for a sensor never seen before.
        self.learned[0] = Some(entry);
    }

    fn remove(&mut self, amp: i32, trans: i32, sample_rate: f32) {
        for slot in self.learned.iter_mut() {
            if slot.is_some_and(|e| e.matches(amp, trans, sample_rate)) {
                *slot = None;
            }
        }
    }

    /// Take the frozen result for the current key, or fall back to "nothing measured yet".
    fn adopt_stored(&mut self) {
        match self.lookup(self.amp, self.trans, self.sample_rate) {
            Some(entry) => {
                self.trims_db = entry.trims_db;
                self.pre_gain_db = entry.pre_gain_db;
                self.state = LearnState::Latched;
            }
            None => {
                self.trims_db = [0.0; 3];
                self.pre_gain_db = [0.0; 3];
                self.state = LearnState::Idle;
            }
        }
    }

    fn load_blob(&mut self, blob: &[f32]) {
        if blob.len() >= BLOB_HEADER && blob[0] == BLOB_MAGIC {
            let count = blob[1].max(0.0) as usize;
            for i in 0..count.min(MAX_LEARNED) {
                let base = BLOB_HEADER + i * ENTRY_LEN;
                if base + ENTRY_LEN > blob.len() {
                    break;
                }
                self.store(LearnedTrim {
                    amp: blob[base] as i32,
                    trans: blob[base + 1] as i32,
                    sample_rate: blob[base + 2],
                    trims_db: [blob[base + 3], blob[base + 4], blob[base + 5]],
                    pre_gain_db: [blob[base + 6], blob[base + 7], blob[base + 8]],
                });
            }
        } else if blob.len() >= 10 && blob[3] > 0.5 {
            // Legacy single-snapshot blob from before per-model storage existed.
            self.store(LearnedTrim {
                amp: blob[0] as i32,
                trans: blob[1] as i32,
                sample_rate: blob[2],
                trims_db: [blob[4], blob[5], blob[6]],
                pre_gain_db: [blob[7], blob[8], blob[9]],
            });
        }
    }

    fn retarget(&mut self) {
        let sr = self.sample_rate.max(1.0) as f64;
        let seconds = self.effective_learn_seconds();
        self.target = (sr * seconds).max(1.0) as usize;
        // A short override must not be blocked by the "trustworthy" minimum, or the pass would
        // reset itself forever.
        self.min_samples = (sr * MIN_SECONDS.min(seconds)) as usize;
        self.timeout = (sr * TIMEOUT_SECONDS) as usize;
    }

    fn begin_learn(&mut self) {
        self.retarget();
        self.settle = ((self.sample_rate.max(1.0) as f64) * SETTLE_SECONDS) as usize;
        self.reset_accumulators();
        for shadow in &mut self.shadows {
            shadow.reset();
        }
        self.state = LearnState::Learning;
    }

    fn abort_learn(&mut self) {
        self.reset_accumulators();
        for shadow in &mut self.shadows {
            shadow.reset();
        }
        self.adopt_stored();
    }

    fn reset_accumulators(&mut self) {
        self.in_pow = 0.0;
        self.out_pow = [0.0; 3];
        self.count = 0;
        self.elapsed = 0;
    }

    fn finish_learn(&mut self) {
        for i in 0..3 {
            let gain = if self.in_pow > 1.0e-12 && self.out_pow[i] > 1.0e-12 {
                (self.in_pow / self.out_pow[i]).sqrt()
            } else {
                1.0
            };
            let db = 20.0 * gain.log10();
            self.trims_db[i] = db.clamp(-MAX_TRIM_DB, MAX_TRIM_DB) as f32;
        }
        self.pre_gain_db = [0.0; 3];
        self.store(LearnedTrim {
            amp: self.amp,
            trans: self.trans,
            sample_rate: self.sample_rate,
            trims_db: self.trims_db,
            pre_gain_db: self.pre_gain_db,
        });
        self.state = LearnState::Latched;
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(sample_rate: f32) -> ColorationAuto {
        let mut auto = ColorationAuto::default();
        auto.configure(1, 1, sample_rate, &[]);
        auto.set_learn_seconds(0.5);
        auto
    }

    fn feed(auto: &mut ColorationAuto, frames: usize) {
        for _ in 0..frames {
            auto.observe(StereoFrame::new(0.25, -0.25));
        }
    }

    /// Settle window plus the 0.5 s measurement, with a little headroom.
    const PASS_FRAMES: usize = 48_000 / 4 + 48_000 / 2 + 32;

    #[test]
    fn engaging_never_starts_a_measurement() {
        let mut auto = engine(48_000.0);
        auto.set_enabled(true);
        assert_eq!(auto.progress(), 0.0);
        assert!(auto.latched_trim_db(1).is_none());
    }

    #[test]
    fn holding_learns_and_disengaging_keeps_the_result() {
        let mut auto = engine(48_000.0);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let learned = auto.latched_trim_db(1).expect("measurement should freeze");

        auto.set_enabled(false);
        assert!(auto.latched_trim_db(1).is_none());
        assert_eq!(auto.progress(), 0.0);

        auto.set_enabled(true);
        assert_eq!(auto.latched_trim_db(1), Some(learned));
    }

    #[test]
    fn disengaging_mid_pass_restores_the_previous_result() {
        let mut auto = engine(48_000.0);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let learned = auto.latched_trim_db(1).expect("measurement should freeze");

        auto.relearn();
        feed(&mut auto, 1_000);
        auto.set_enabled(false);
        auto.set_enabled(true);

        assert_eq!(auto.latched_trim_db(1), Some(learned));
    }

    #[test]
    fn each_model_keeps_its_own_result() {
        let mut auto = engine(48_000.0);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let learned = auto.latched_trim_db(1).expect("measurement should freeze");

        auto.configure(2, 1, 48_000.0, &[]);
        assert!(
            auto.latched_trim_db(1).is_none(),
            "new model starts unmeasured"
        );
        assert_eq!(auto.progress(), 0.0, "model switch must not start learning");

        auto.configure(1, 1, 48_000.0, &[]);
        assert_eq!(auto.latched_trim_db(1), Some(learned));
    }

    #[test]
    fn clearing_forgets_only_the_current_model() {
        let mut auto = engine(48_000.0);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let learned = auto.latched_trim_db(1).expect("measurement should freeze");

        auto.configure(2, 1, 48_000.0, &[]);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let other = auto.latched_trim_db(1).expect("second model should freeze");

        auto.clear_learned();
        assert!(auto.latched_trim_db(1).is_none());
        assert!(auto.snapshot_blob().len() < BLOB_HEADER + 2 * ENTRY_LEN);

        auto.configure(1, 1, 48_000.0, &[]);
        assert_eq!(auto.latched_trim_db(1), Some(learned));
        auto.configure(2, 1, 48_000.0, &[]);
        assert!(auto.latched_trim_db(1).is_none());
        let _ = other;
    }

    #[test]
    fn changing_the_learn_time_retargets_a_running_pass() {
        let mut auto = engine(48_000.0);
        auto.set_learn_seconds(12.0);
        auto.relearn();
        feed(&mut auto, 40_000);
        assert!(
            auto.latched_trim_db(1).is_none(),
            "a 12 s pass must still be measuring"
        );

        auto.set_learn_seconds(0.5);
        feed(&mut auto, 10);
        assert!(
            auto.latched_trim_db(1).is_some(),
            "shortening the time must finish the pass"
        );
    }

    #[test]
    fn blob_round_trips_every_model() {
        let mut auto = engine(48_000.0);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        auto.configure(3, 2, 48_000.0, &[]);
        auto.relearn();
        feed(&mut auto, PASS_FRAMES);
        let blob = auto.snapshot_blob();

        let mut restored = ColorationAuto::default();
        restored.configure(3, 2, 48_000.0, &blob);
        restored.set_enabled(true);
        assert!(restored.latched_trim_db(1).is_some());
        restored.configure(1, 1, 48_000.0, &blob);
        assert!(restored.latched_trim_db(1).is_some());
    }
}
