//! Knob primitives: concentric dual-concentric rotor, large single knob, and the small
//! stepped pot with optional card, lift and lit/unlit faces.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Canvas, Color, LineCap, Paint, Path, Renderer, Solidity};
use std::f32::consts::PI;

use super::card::{draw_plate_well_160x200, draw_plate_well_80x100};
use super::dseg::draw_dseg_display;
use super::palette::{accent_magenta, is_light, silk_muted};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

pub const TICK_ANGLES_11: [f32; 11] = [
    -135.0, -108.0, -81.0, -54.0, -27.0, 0.0, 27.0, 54.0, 81.0, 108.0, 135.0,
];
pub const CONCENTRIC_GAIN_DEGS: [f32; 9] =
    [-90.0, -67.5, -45.0, -22.5, 0.0, 22.5, 45.0, 67.5, 90.0];
pub const CONCENTRIC_FREQ_POLAR_DEGS: [f32; 6] = [150.0, 126.0, 102.0, 78.0, 54.0, 30.0];

pub fn draw_knob_concentric<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    readout_str: &str,
    active_f_idx: usize,
    g_angle_deg: f32,
    active_g_idx: usize,
    is_on: bool,
    fonts: ThemeFonts,
) {
    draw_plate_well_160x200(canvas, cx - 68.0, cy - 108.0, 136.0, 172.0);
    draw_dseg_display(
        canvas,
        cx,
        cy - 78.0,
        84.0,
        32.0,
        6,
        readout_str,
        8.0,
        fonts,
    );

    // Upper half-circle arc
    let r_arc = 54.0;
    let mut arc = Path::new();
    arc.arc(cx, cy, r_arc, -PI, 0.0, Solidity::Hole);
    let mut arc_p = Paint::color(if is_light() {
        Color::rgba(0x3A, 0x3A, 0x44, 110)
    } else {
        Color::rgba(0xEC, 0xE9, 0xE1, 90)
    });
    arc_p.set_line_width(0.9);
    canvas.stroke_path(&arc, &arc_p);

    // 9 Gain ticks (-90..+90 deg)
    for (idx, &deg) in CONCENTRIC_GAIN_DEGS.iter().enumerate() {
        let rad = (deg - 90.0) * PI / 180.0;
        let x1 = cx + 46.5 * rad.cos();
        let y1 = cy + 46.5 * rad.sin();
        let x2 = cx + 54.0 * rad.cos();
        let y2 = cy + 54.0 * rad.sin();
        let is_sel = (idx == active_g_idx) && is_on;
        let mut t = Path::new();
        t.move_to(x1, y1);
        t.line_to(x2, y2);
        let mut tp = Paint::color(if is_sel {
            accent_magenta()
        } else if is_light() {
            Color::rgba(0x3A, 0x3A, 0x44, 160)
        } else {
            Color::rgba(0xEC, 0xE9, 0xE1, 115)
        });
        tp.set_line_width(1.4);
        tp.set_line_cap(LineCap::Round);
        canvas.stroke_path(&t, &tp);
    }

    // 6 Lower Frequency dots
    for (idx, &polar_deg) in CONCENTRIC_FREQ_POLAR_DEGS.iter().enumerate() {
        let rad = polar_deg * PI / 180.0;
        let fx = cx + 54.0 * rad.cos();
        let fy = cy + 54.0 * rad.sin();
        let is_sel = (idx == active_f_idx) && is_on;
        let mut d = Path::new();
        d.circle(fx, fy, if is_sel { 2.2 } else { 1.4 });
        let col = if is_sel {
            accent_magenta()
        } else if is_light() {
            Color::rgba(0x60, 0x62, 0x6C, 160)
        } else {
            Color::rgba(0x8A, 0x8A, 0x90, 115)
        };
        canvas.fill_path(&d, &Paint::color(col));
    }

    // Outer Knurled Collar (R = 46.0)
    let light = is_light();
    let mut sh = Path::new();
    sh.circle(cx, cy + 2.5, 47.0);
    canvas.fill_path(&sh, &Paint::color(if light {
        Color::rgba(0, 0, 0, 40)
    } else {
        Color::rgba(0, 0, 0, 130)
    }));

    let mut collar = Path::new();
    collar.circle(cx, cy, 46.0);
    let collar_grad = if light {
        Paint::linear_gradient(
            cx,
            cy - 46.0,
            cx,
            cy + 46.0,
            Color::rgb(0xEB, 0xE7, 0xDF),
            Color::rgb(0xC8, 0xC4, 0xBC),
        )
    } else {
        Paint::linear_gradient(
            cx,
            cy - 46.0,
            cx,
            cy + 46.0,
            Color::rgb(0x42, 0x42, 0x48),
            Color::rgb(0x16, 0x16, 0x19),
        )
    };
    canvas.fill_path(&collar, &collar_grad);
    let mut cs = Paint::color(if light {
        Color::rgb(0x9E, 0x9A, 0x92)
    } else {
        Color::rgb(0x17, 0x17, 0x1A)
    });
    cs.set_line_width(1.0);
    canvas.stroke_path(&collar, &cs);

    // Protruding 2px Frequency Pointer at f_angle
    let f_deg = CONCENTRIC_FREQ_POLAR_DEGS[active_f_idx.min(5)];
    let f_rad = f_deg * PI / 180.0;
    let px1 = cx + 40.0 * f_rad.cos();
    let py1 = cy + 40.0 * f_rad.sin();
    let px2 = cx + 48.0 * f_rad.cos();
    let py2 = cy + 48.0 * f_rad.sin();

    let mut ptr_sh = Path::new();
    ptr_sh.move_to(px1, py1);
    ptr_sh.line_to(px2, py2);
    let mut psp = Paint::color(Color::rgba(0, 0, 0, 180));
    psp.set_line_width(4.0);
    psp.set_line_cap(LineCap::Round);
    canvas.stroke_path(&ptr_sh, &psp);

    let mut ptr = Path::new();
    ptr.move_to(px1, py1);
    ptr.line_to(px2, py2);
    let mut pp = Paint::color(if is_on {
        accent_magenta()
    } else {
        Color::rgb(0x55, 0x55, 0x5E)
    });
    pp.set_line_width(2.6);
    pp.set_line_cap(LineCap::Round);
    canvas.stroke_path(&ptr, &pp);

    // Inner Main Rotor (R = 40.0)
    let light = is_light();
    let mut inner = Path::new();
    inner.circle(cx, cy, 40.0);
    let inner_grad = if light {
        Paint::linear_gradient(
            cx,
            cy - 40.0,
            cx,
            cy + 40.0,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD8, 0xD8, 0xDE),
        )
    } else {
        Paint::linear_gradient(
            cx,
            cy - 40.0,
            cx,
            cy + 40.0,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&inner, &inner_grad);
    let mut is = Paint::color(if light {
        Color::rgb(0xB4, 0xB4, 0xBC)
    } else {
        Color::rgb(0x18, 0x18, 0x1C)
    });
    is.set_line_width(0.9);
    canvas.stroke_path(&inner, &is);

    // Rotating Gain Indicator Notch
    canvas.save();
    canvas.translate(cx, cy);
    canvas.rotate(g_angle_deg * PI / 180.0);
    let mut n_sh = Path::new();
    n_sh.rounded_rect(-2.5, -34.5, 5.0, 21.0, 2.5);
    canvas.fill_path(&n_sh, &Paint::color(Color::rgba(0, 0, 0, 128)));
    let mut notch = Path::new();
    notch.rounded_rect(-2.0, -34.0, 4.0, 20.0, 2.0);
    canvas.fill_path(
        &notch,
        &Paint::color(if is_on {
            accent_magenta()
        } else if light {
            Color::rgb(0x88, 0x88, 0x92)
        } else {
            Color::rgb(0x6E, 0x6E, 0x77)
        }),
    );
    canvas.restore();
}

pub fn draw_knob_large<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    readout_str: &str,
    angle_deg: f32,
    fonts: ThemeFonts,
) {
    draw_knob_large_lit(canvas, cx, cy, readout_str, angle_deg, fonts, true);
}

pub fn draw_knob_large_lit<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    readout_str: &str,
    angle_deg: f32,
    fonts: ThemeFonts,
    lit: bool,
) {
    draw_plate_well_160x200(canvas, cx - 68.0, cy - 108.0, 136.0, 172.0);
    draw_dseg_display(
        canvas,
        cx,
        cy - 78.0,
        84.0,
        32.0,
        6,
        readout_str,
        8.0,
        fonts,
    );

    let r_in = 46.5;
    let r_arc = 54.0;
    let mut best_idx = 0;
    let mut min_d = 999.0;
    for (i, &deg) in TICK_ANGLES_11.iter().enumerate() {
        let d = (deg - angle_deg).abs();
        if d < min_d {
            min_d = d;
            best_idx = i;
        }
    }

    let light = is_light();
    let a_start = (-135.0 - 90.0) * PI / 180.0;
    let a_end = (135.0 - 90.0) * PI / 180.0;
    let mut arc = Path::new();
    arc.arc(cx, cy, r_arc, a_start, a_end, Solidity::Hole);
    let mut arc_p = Paint::color(if light {
        Color::rgba(0x3A, 0x3A, 0x44, 110)
    } else {
        Color::rgba(0xEC, 0xE9, 0xE1, 90)
    });
    arc_p.set_line_width(0.9);
    canvas.stroke_path(&arc, &arc_p);

    for (idx, &deg) in TICK_ANGLES_11.iter().enumerate() {
        let rad = (deg - 90.0) * PI / 180.0;
        let x1 = cx + r_in * rad.cos();
        let y1 = cy + r_in * rad.sin();
        let x2 = cx + r_arc * rad.cos();
        let y2 = cy + r_arc * rad.sin();
        let is_sel = (idx == best_idx) && lit;
        let mut t = Path::new();
        t.move_to(x1, y1);
        t.line_to(x2, y2);
        let mut tp = Paint::color(if is_sel {
            accent_magenta()
        } else if light {
            Color::rgba(0x3A, 0x3A, 0x44, 160)
        } else {
            Color::rgba(0xEC, 0xE9, 0xE1, 115)
        });
        tp.set_line_width(1.4);
        tp.set_line_cap(LineCap::Round);
        canvas.stroke_path(&t, &tp);
    }

    let mut sh = Path::new();
    sh.circle(cx, cy + 2.5, 47.0);
    canvas.fill_path(&sh, &Paint::color(if light {
        Color::rgba(0, 0, 0, 60)
    } else {
        Color::rgba(0, 0, 0, 130)
    }));

    let mut rotor = Path::new();
    rotor.circle(cx, cy, 46.0);
    let r_grad = if light {
        Paint::linear_gradient(
            cx,
            cy - 46.0,
            cx,
            cy + 46.0,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD8, 0xD8, 0xDE),
        )
    } else {
        Paint::linear_gradient(
            cx,
            cy - 46.0,
            cx,
            cy + 46.0,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&rotor, &r_grad);
    let mut rs = Paint::color(if light {
        Color::rgb(0xB4, 0xB4, 0xBC)
    } else {
        Color::rgb(0x18, 0x18, 0x1C)
    });
    rs.set_line_width(1.0);
    canvas.stroke_path(&rotor, &rs);

    canvas.save();
    canvas.translate(cx, cy);
    canvas.rotate(angle_deg * PI / 180.0);
    let mut n_sh = Path::new();
    n_sh.rounded_rect(-2.5, -40.5, 5.0, 23.0, 2.5);
    canvas.fill_path(&n_sh, &Paint::color(Color::rgba(0, 0, 0, 128)));
    let mut notch = Path::new();
    notch.rounded_rect(-2.0, -40.0, 4.0, 22.0, 2.0);
    canvas.fill_path(
        &notch,
        &Paint::color(if lit {
            accent_magenta()
        } else if light {
            Color::rgb(0x88, 0x88, 0x92)
        } else {
            Color::rgb(0x6E, 0x6E, 0x77)
        }),
    );
    canvas.restore();
}

pub fn draw_knob_small<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
) {
    draw_knob_small_ticks(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle_deg,
        with_card,
        fonts,
        &TICK_ANGLES_11,
    );
}

/// Small knob with `count` evenly spaced stepped positions (-135..+135 deg); `idx` is the active position.
pub fn draw_knob_small_n<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    count: usize,
    idx: usize,
    fonts: ThemeFonts,
) {
    let count = count.max(2);
    let ticks: Vec<f32> = (0..count)
        .map(|i| -135.0 + 270.0 * (i as f32) / ((count - 1) as f32))
        .collect();
    let angle = ticks[idx.min(count - 1)];
    draw_knob_small_ticks(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle,
        true,
        fonts,
        &ticks,
    );
}

pub fn draw_knob_small_ticks<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
    ticks: &[f32],
) {
    draw_knob_small_ticks_lift(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle_deg,
        with_card,
        fonts,
        ticks,
        0.0,
    );
}

/// Small pot with a push-pull "lift": `lift` 0..1 pulls the cap toward the viewer (slightly
/// larger, a couple of pixels higher, deeper shadow) the way a push-pull pot latches out.
pub fn draw_knob_small_lifted<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
    lift: f32,
) {
    let ticks: Vec<f32> = (0..11)
        .map(|i| -135.0 + 270.0 * (i as f32) / 10.0)
        .collect();
    draw_knob_small_ticks_lift(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle_deg,
        with_card,
        fonts,
        &ticks,
        lift,
    );
}

pub fn draw_knob_small_ticks_lift<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
    ticks: &[f32],
    lift: f32,
) {
    draw_knob_small_ticks_lift_lit(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle_deg,
        with_card,
        fonts,
        ticks,
        lift,
        true,
    );
}

/// Same as [`draw_knob_small_ticks_lift`] but with an "off" (unlit) face: the pointer notch and
/// the readout drop the accent colour so a servo-driven control reads as passive.
pub fn draw_knob_small_unlit<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
) {
    let ticks: Vec<f32> = (0..11)
        .map(|i| -135.0 + 270.0 * (i as f32) / 10.0)
        .collect();
    draw_knob_small_ticks_lift_lit(
        canvas,
        cx,
        cy,
        label,
        readout_val,
        angle_deg,
        with_card,
        fonts,
        &ticks,
        0.0,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_knob_small_ticks_lift_lit<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    label: &str,
    readout_val: &str,
    angle_deg: f32,
    with_card: bool,
    fonts: ThemeFonts,
    ticks: &[f32],
    lift: f32,
    lit: bool,
) {
    let lift = lift.clamp(0.0, 1.0);
    let (y_top, y_bot) = if with_card {
        draw_plate_well_80x100(canvas, cx - 40.0, cy - 54.0);
        (cy - 38.5, cy + 32.0)
    } else {
        (cy - 38.0, cy + 31.5)
    };

    let mut best_idx = 0;
    let mut min_d = 999.0;
    for (i, &deg) in ticks.iter().enumerate() {
        let d = (deg - angle_deg).abs();
        if d < min_d {
            min_d = d;
            best_idx = i;
        }
    }

    let light = is_light();
    let r_in = 24.0;
    let r_arc = 30.5;
    let a_start = (-135.0 - 90.0) * PI / 180.0;
    let a_end = (135.0 - 90.0) * PI / 180.0;
    let mut arc = Path::new();
    arc.arc(cx, cy, r_arc, a_start, a_end, Solidity::Hole);
    let mut arc_p = Paint::color(if light {
        Color::rgba(0x3A, 0x3A, 0x44, 110)
    } else {
        Color::rgba(0xEC, 0xE9, 0xE1, 90)
    });
    arc_p.set_line_width(0.9);
    canvas.stroke_path(&arc, &arc_p);

    for (idx, &deg) in ticks.iter().enumerate() {
        let rad = (deg - 90.0) * PI / 180.0;
        let x1 = cx + r_in * rad.cos();
        let y1 = cy + r_in * rad.sin();
        let x2 = cx + r_arc * rad.cos();
        let y2 = cy + r_arc * rad.sin();
        let is_sel = idx == best_idx;
        let is_major = deg == 0.0 || deg.abs() == 135.0;
        let mut t = Path::new();
        t.move_to(x1, y1);
        t.line_to(x2, y2);
        let col = if is_sel {
            if lit {
                accent_magenta()
            } else if light {
                Color::rgba(0x3A, 0x3A, 0x44, 210)
            } else {
                Color::rgba(0xEC, 0xE9, 0xE1, 190)
            }
        } else if is_major {
            if light {
                Color::rgba(0x3A, 0x3A, 0x44, 190)
            } else {
                Color::rgba(0xEC, 0xE9, 0xE1, 190)
            }
        } else {
            if light {
                Color::rgba(0x3A, 0x3A, 0x44, 120)
            } else {
                Color::rgba(0xEC, 0xE9, 0xE1, 115)
            }
        };
        let mut tp = Paint::color(col);
        tp.set_line_width(1.4);
        tp.set_line_cap(LineCap::Round);
        canvas.stroke_path(&t, &tp);
    }

    // Concentric circular base skirt (R = 23.0)
    let mut skirt = Path::new();
    skirt.circle(cx, cy, 23.0);
    let sk_grad = if light {
        Paint::linear_gradient(
            cx,
            cy - 23.0,
            cx,
            cy + 23.0,
            Color::rgb(0xE6, 0xE3, 0xDC),
            Color::rgb(0xD0, 0xCC, 0xC4),
        )
    } else {
        Paint::linear_gradient(
            cx,
            cy - 23.0,
            cx,
            cy + 23.0,
            Color::rgb(0x2C, 0x2C, 0x32),
            Color::rgb(0x16, 0x16, 0x19),
        )
    };
    canvas.fill_path(&skirt, &sk_grad);
    let mut sk_s = Paint::color(if light {
        Color::rgb(0xB0, 0xAC, 0xA4)
    } else {
        Color::rgb(0x10, 0x10, 0x14)
    });
    sk_s.set_line_width(0.9);
    canvas.stroke_path(&skirt, &sk_s);

    // Push-pull latch: cast a deeper shadow under the raised cap.
    if lift > 0.0 {
        let mut sh = Path::new();
        sh.circle(cx, cy + 1.5 + 3.0 * lift, 23.2 + 2.2 * lift);
        canvas.fill_path(
            &sh,
            &Paint::color(Color::rgba(0x00, 0x00, 0x00, (110.0 * lift) as u8)),
        );
    }

    // Rotating 6-Flute Scalloped Head + Pointer Notch
    // NOTE: Canvas is translated and scaled, but NOT rotated here so that lighting gradients
    // and shadows stay completely static in screen space while the flutes and pointer turn.
    canvas.save();
    canvas.translate(cx, cy - 2.2 * lift);
    let head_scale = 1.0 + 0.09 * lift;
    canvas.scale(head_scale, head_scale);
    let rad = angle_deg * PI / 180.0;

    let mut scallop = Path::new();
    let num_pts = 72;
    for i in 0..=num_pts {
        let a = (i as f32 / num_pts as f32) * 2.0 * PI - PI * 0.5;
        let flute = (6.0 * (a + PI * 0.5)).cos(); // +1 at 0, 60, 120.. deg; -1 at 30, 90..
        let r = if flute > 0.65 {
            23.0
        } else {
            20.2 + 2.8 * flute
        };
        let rot_a = a + rad;
        let px = r * rot_a.cos();
        let py = r * rot_a.sin();
        if i == 0 {
            scallop.move_to(px, py);
        } else {
            scallop.line_to(px, py);
        }
    }
    scallop.close();

    // Static overhead lighting gradient (top highlight, bottom shadow)
    let sc_grad = if light {
        Paint::linear_gradient(
            0.0,
            -23.0,
            0.0,
            23.0,
            Color::rgb(0xFD, 0xFD, 0xFD),
            Color::rgb(0xD8, 0xD8, 0xDE),
        )
    } else {
        Paint::linear_gradient(
            0.0,
            -23.0,
            0.0,
            23.0,
            Color::rgb(0x3A, 0x3A, 0x40),
            Color::rgb(0x20, 0x20, 0x24),
        )
    };
    canvas.fill_path(&scallop, &sc_grad);
    let sc_edge = if lift > 0.0 {
        let v = (0x18 as f32 + 54.0 * lift) as u8;
        Color::rgb(v, v, (v as f32 + 4.0).min(255.0) as u8)
    } else if light {
        Color::rgb(0xB4, 0xB4, 0xBC)
    } else {
        Color::rgb(0x18, 0x18, 0x1C)
    };
    let mut sc_s = Paint::color(sc_edge);
    sc_s.set_line_width(0.9);
    canvas.stroke_path(&scallop, &sc_s);

    // Rotating Magenta Pointer Notch on front crest (neutral when the knob reads as "off")
    canvas.save();
    canvas.rotate(rad);
    let mut notch = Path::new();
    notch.rounded_rect(-1.3, -21.0, 2.6, 11.5, 1.3);
    canvas.fill_path(
        &notch,
        &Paint::color(if lit {
            accent_magenta()
        } else {
            Color::rgb(0x8A, 0x8A, 0x92)
        }),
    );
    canvas.restore();

    canvas.restore();

    draw_text_centered(
        canvas,
        cx,
        y_top,
        label,
        fonts.bold_paint(silk_muted(), 9.5),
    );
    draw_text_centered(
        canvas,
        cx,
        y_bot,
        readout_val,
        fonts.bold_paint(if lit { accent_magenta() } else { silk_muted() }, 9.5),
    );
}
