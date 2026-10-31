use std::sync::Arc;

use nih_plug::prelude::*;

use crate::dsp::chain::MasteringChain;
use crate::parameters::MasteraidParams;

pub struct Masteraid {
    params: Arc<MasteraidParams>,
    chain: MasteringChain,
}

impl Default for Masteraid {
    fn default() -> Self {
        // Load the on-disk debug calibration here, on the plugin's construction thread, so the
        // audio thread never has to touch the file.
        let _ = crate::debug_calibration::calibration();
        Self {
            params: Arc::new(MasteraidParams::default()),
            chain: MasteringChain::default(),
        }
    }
}

impl Plugin for Masteraid {
    const NAME: &'static str = "Masteraid";
    const VENDOR: &'static str = "MQN DSP";
    const URL: &'static str = "https://github.com/xxlintux/mqndsp-masteraid";
    const EMAIL: &'static str = "contact@mqndsp.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let meters = self.chain.meters.clone();
        nih_plug_vizia::create_vizia_editor(
            self.params.editor_state.clone(),
            nih_plug_vizia::ViziaTheming::Custom,
            move |cx, gui_context| {
                crate::gui::EqEditorView::new(cx, params.clone(), meters.clone(), gui_context);
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.chain.set_sample_rate(buffer_config.sample_rate);
        self.chain.reset();
        true
    }

    fn reset(&mut self) {
        self.chain.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.chain.process(buffer, &self.params);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Masteraid {
    const CLAP_ID: &'static str = "com.mqn.masteraid";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Analog modular mastering processor");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::AudioEffect, ClapFeature::Mastering];
}

impl Vst3Plugin for Masteraid {
    const VST3_CLASS_ID: [u8; 16] = *b"MasteraidPlugin!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Mastering];
}
