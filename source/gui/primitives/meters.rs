//! Meter primitives: the analogue VU, the dual input ladder and the LUFS/true-peak display.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer, Solidity};
use std::f32::consts::PI;

use super::dseg::{draw_dseg_display_left, draw_dseg_display_tagged};
use super::palette::{dseg_active_pink, is_light, silk_muted};
use super::text::draw_text_centered;
use crate::gui::theme::ThemeFonts;

pub fn draw_vu_meter<T: Renderer>(
    canvas: &mut Canvas<T>,
    vx: f32,
    vy: f32,
    vtitle: &str,
    needle_deg: f32,
    fonts: ThemeFonts,
) {
    let pivot_x = vx + 100.0;
    let pivot_y = vy + 118.0;
    let r_arc = 86.0;
    let r_label = 96.5;

    let light = is_light();
    let mut outer = Path::new();
    outer.rounded_rect(vx, vy, 200.0, 132.0, 16.0);
    let (outer_fill, outer_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&outer, &Paint::color(outer_fill));
    let mut os = Paint::color(outer_stroke);
    os.set_line_width(1.0);
    canvas.stroke_path(&outer, &os);

    let mut trench = Path::new();
    trench.rounded_rect(vx + 3.0, vy + 3.0, 194.0, 126.0, 13.5);
    canvas.fill_path(&trench, &Paint::color(if light {
        Color::rgb(0xB8, 0xB5, 0xAE)
    } else {
        Color::rgb(0x07, 0x07, 0x09)
    }));

    let mut cav = Path::new();
    cav.rounded_rect(vx + 7.0, vy + 7.0, 186.0, 118.0, 10.5);
    let cav_grad = if light {
        Paint::linear_gradient(
            vx,
            vy + 7.0,
            vx,
            vy + 125.0,
            Color::rgb(0xEC, 0xD8, 0xE8),
            Color::rgb(0xDC, 0xC4, 0xDA),
        )
    } else {
        Paint::linear_gradient(
            vx,
            vy + 7.0,
            vx,
            vy + 125.0,
            Color::rgb(0x16, 0x06, 0x0E),
            Color::rgb(0x0D, 0x03, 0x08),
        )
    };
    canvas.fill_path(&cav, &cav_grad);

    canvas.save();
    canvas.scissor(vx + 7.0, vy + 7.0, 186.0, 118.0);

    let a_s = (-44.0 - 90.0) * PI / 180.0;
    let a_m = (11.0 - 90.0) * PI / 180.0;
    let a_e = (44.0 - 90.0) * PI / 180.0;

    let mut safe_arc = Path::new();
    safe_arc.arc(pivot_x, pivot_y, r_arc, a_s, a_m, Solidity::Hole);
    let mut sp = Paint::color(if light {
        Color::rgb(0x4A, 0x42, 0x46)
    } else {
        Color::rgba(0xF8, 0xDC, 0xE9, 224)
    });
    sp.set_line_width(1.35);
    canvas.stroke_path(&safe_arc, &sp);

    let mut over_arc = Path::new();
    over_arc.arc(pivot_x, pivot_y, r_arc, a_m, a_e, Solidity::Hole);
    let mut op = Paint::color(Color::rgb(0xFF, 0x1E, 0x8E));
    op.set_line_width(2.2);
    canvas.stroke_path(&over_arc, &op);

    let ticks: [(f32, &str, bool, bool); 9] = [
        (-44.0, "-20", true, false),
        (-33.0, "-10", false, false),
        (-22.0, "-7", false, false),
        (-11.0, "-5", false, false),
        (0.0, "-3", true, false),
        (11.0, "0", true, false),
        (22.0, "+1", false, true),
        (33.0, "+2", false, true),
        (44.0, "+3", true, true),
    ];

    for (deg, val, is_major, is_mag) in ticks {
        let len = if is_major { 8.5 } else { 5.5 };
        let rad = (deg - 90.0) * PI / 180.0;
        let x_out = pivot_x + r_arc * rad.cos();
        let y_out = pivot_y + r_arc * rad.sin();
        let x_in = pivot_x + (r_arc - len) * rad.cos();
        let y_in = pivot_y + (r_arc - len) * rad.sin();

        let mut tl = Path::new();
        tl.move_to(x_out, y_out);
        tl.line_to(x_in, y_in);
        let mut tp = Paint::color(if is_mag {
            Color::rgb(0xFF, 0x1E, 0x8E)
        } else if light {
            Color::rgb(0x4A, 0x42, 0x46)
        } else {
            Color::rgba(0xF8, 0xDC, 0xE9, 215)
        });
        tp.set_line_width(1.4);
        canvas.stroke_path(&tl, &tp);

        let lx = pivot_x + r_label * rad.cos();
        let ly = pivot_y + r_label * rad.sin();
        let tcol = if is_mag {
            dseg_active_pink()
        } else if light {
            Color::rgb(0x5A, 0x50, 0x54)
        } else {
            Color::rgb(0xF2, 0xC8, 0xDC)
        };
        draw_text_centered(canvas, lx, ly, val, fonts.bold_paint(tcol, 7.2));
    }

    draw_text_centered(
        canvas,
        pivot_x,
        vy + 82.0,
        "VU",
        fonts.bold_paint(if light {
            Color::rgba(0x7A, 0x52, 0x68, 204)
        } else {
            Color::rgba(0xD4, 0x84, 0xAC, 204)
        }, 11.0),
    );
    draw_text_centered(
        canvas,
        pivot_x,
        vy + 94.0,
        vtitle,
        fonts.bold_paint(dseg_active_pink(), 7.8),
    );

    // Needle
    let light = is_light();
    canvas.save();
    canvas.translate(pivot_x, pivot_y);
    canvas.rotate(needle_deg * PI / 180.0);
    let mut n_sh = Path::new();
    n_sh.rounded_rect(-0.2, -90.0, 2.8, 90.0, 1.4);
    canvas.fill_path(&n_sh, &Paint::color(if light {
        Color::rgba(0, 0, 0, 50)
    } else {
        Color::rgba(0, 0, 0, 150)
    }));
    let mut needle = Path::new();
    needle.rounded_rect(-1.4, -90.0, 2.8, 90.0, 1.4);
    canvas.fill_path(&needle, &Paint::color(if light {
        Color::rgb(0x2A, 0x1E, 0x24)
    } else {
        Color::rgb(0xDF, 0xC5, 0xD2)
    }));
    canvas.restore();

    let mut hub = Path::new();
    hub.circle(pivot_x, pivot_y + 1.0, 8.0);
    if light {
        let hub_grad = Paint::radial_gradient(
            pivot_x - 2.0,
            pivot_y - 1.0,
            1.5,
            8.0,
            Color::rgb(0xFD, 0xFB, 0xF7),
            Color::rgb(0xB8, 0xB4, 0xAC),
        );
        canvas.fill_path(&hub, &hub_grad);
        let mut hs = Paint::color(Color::rgb(0x8A, 0x86, 0x7E));
        hs.set_line_width(1.0);
        canvas.stroke_path(&hub, &hs);
    } else {
        canvas.fill_path(&hub, &Paint::color(Color::rgb(0x26, 0x18, 0x20)));
    }

    canvas.restore();

    // Front glass glare & rim
    let mut glare = Path::new();
    glare.rounded_rect(vx + 9.5, vy + 9.0, 181.0, 10.0, 5.0);
    canvas.fill_path(&glare, &Paint::color(Color::rgba(255, 255, 255, 13)));

    let mut rim = Path::new();
    rim.rounded_rect(vx + 7.0, vy + 7.0, 186.0, 118.0, 10.5);
    let mut rs = Paint::color(Color::rgba(0xFF, 0x00, 0x7F, 66));
    rs.set_line_width(0.9);
    canvas.stroke_path(&rim, &rs);
}

pub fn draw_input_ladder<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    peak_str: &str,
    peak_banner: Option<&str>,
    rms_str: &str,
    rms_banner: Option<&str>,
    lit_l: usize,
    lit_r: usize,
    is_rms_ladder: bool,
    fonts: ThemeFonts,
) {
    let cx = x + w * 0.5;

    // 1. Top Readout: PEAK (P tag)
    if let Some(banner) = peak_banner {
        draw_dseg_display_left(
            canvas,
            cx,
            y + 13.0,
            w,
            26.0,
            6,
            banner,
            7.5,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            cx,
            y + 13.0,
            w,
            26.0,
            6,
            "P",
            peak_str,
            7.5,
            fonts,
        );
    }

    // 2. Center Ladder Frame: 112px tall, from y + 32.0 to y + 144.0
    let ly = y + 32.0;
    let h = 112.0;
    let light = is_light();
    let mut f_out = Path::new();
    f_out.rounded_rect(x, ly, w, h, 7.0);
    let (f_fill, f_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&f_out, &Paint::color(f_fill));
    let mut s_out = Paint::color(f_stroke);
    s_out.set_line_width(1.0);
    canvas.stroke_path(&f_out, &s_out);

    let mut f_tr = Path::new();
    f_tr.rounded_rect(x + 1.5, ly + 1.5, w - 3.0, h - 3.0, 5.5);
    canvas.fill_path(&f_tr, &Paint::color(if light {
        Color::rgb(0xB8, 0xB5, 0xAE)
    } else {
        Color::rgb(0x07, 0x07, 0x09)
    }));

    let mut f_lens = Path::new();
    f_lens.rounded_rect(x + 3.0, ly + 3.0, w - 6.0, h - 6.0, 4.5);
    let lens_grad = if light {
        Paint::linear_gradient(
            x,
            ly + 3.0,
            x,
            ly + h - 3.0,
            Color::rgb(0xEB, 0xD8, 0xE6),
            Color::rgb(0xDC, 0xC6, 0xDA),
        )
    } else {
        Paint::linear_gradient(
            x,
            ly + 3.0,
            x,
            ly + h - 3.0,
            Color::rgb(0x16, 0x06, 0x0E),
            Color::rgb(0x0D, 0x03, 0x08),
        )
    };
    canvas.fill_path(&f_lens, &lens_grad);
    let mut s_lens = Paint::color(if light {
        Color::rgba(0xFF, 0x00, 0x7F, 71)
    } else {
        Color::rgba(0xFF, 0x00, 0x7F, 56)
    });
    s_lens.set_line_width(0.8);
    canvas.stroke_path(&f_lens, &s_lens);

    let num_segs = 12usize;
    let top_pad = 7.0;
    let bot_pad = 7.0;
    let usable_h = h - top_pad - bot_pad;
    let step_y = usable_h / (num_segs as f32);
    let seg_h = step_y * 0.64;
    let bar_w = 16.0;
    let lx = x + 12.0;
    let rx = x + w - 12.0 - bar_w;

    let scale_ticks: [(usize, &str); 6] = [
        (0, "0"),
        (2, "6"),
        (4, "12"),
        (6, "18"),
        (8, "24"),
        (11, "36"),
    ];

    for i in 0..num_segs {
        let seg_from_bottom = num_segs - 1 - i;
        let sy = ly + top_pad + (i as f32) * step_y;

        for (bx, lit_count) in [(lx, lit_l), (rx, lit_r)] {
            let mut seg = Path::new();
            seg.rounded_rect(bx, sy, bar_w, seg_h, 1.2);
            if seg_from_bottom < lit_count {
                canvas.fill_path(&seg, &Paint::color(dseg_active_pink()));
            } else {
                canvas.fill_path(&seg, &Paint::color(Color::rgba(0xFF, 0x00, 0x7F, 41)));
            }
        }
    }

    // Printed benchmark scale markings down the center channel
    for &(idx, lbl) in &scale_ticks {
        let sy = ly + top_pad + (idx as f32) * step_y + seg_h * 0.5;
        draw_text_centered(
            canvas,
            cx,
            sy,
            lbl,
            fonts.bold_paint(
                if light {
                    Color::rgba(0x20, 0x22, 0x2A, 220)
                } else {
                    Color::rgba(0xEC, 0xE9, 0xE1, 140)
                },
                6.5,
            ),
        );
    }

    // Ladder Mode indicator (click ladder to toggle)
    let mode_lbl = if is_rms_ladder { "RMS" } else { "PK" };
    draw_text_centered(
        canvas,
        cx,
        ly + h - 4.5,
        mode_lbl,
        fonts.regular_paint(
            if light {
                Color::rgba(0x20, 0x22, 0x2A, 200)
            } else {
                Color::rgba(0xEC, 0xE9, 0xE1, 130)
            },
            7.0,
        ),
    );

    // 3. Bottom Readout: RMS (R tag)
    let bot_cy = ly + h + 6.0 + 13.0;
    if let Some(banner) = rms_banner {
        draw_dseg_display_left(
            canvas,
            cx,
            bot_cy,
            w,
            26.0,
            6,
            banner,
            7.5,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            cx,
            bot_cy,
            w,
            26.0,
            6,
            "R",
            rms_str,
            7.5,
            fonts,
        );
    }
}

pub fn draw_limiter_ladder<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    lufs_tag: &str,
    lufs_str: &str,
    lufs_banner: Option<&str>,
    bot_tag: &str,
    bot_str: &str,
    bot_banner: Option<&str>,
    out_segs: usize,
    gr_segs: usize,
    fonts: ThemeFonts,
) {
    let cx = x + w * 0.5;

    // 1. Top Readout: LUFS
    if let Some(banner) = lufs_banner {
        draw_dseg_display_left(
            canvas,
            cx,
            y + 13.0,
            w,
            26.0,
            6,
            banner,
            7.5,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            cx,
            y + 13.0,
            w,
            26.0,
            6,
            lufs_tag,
            lufs_str,
            7.5,
            fonts,
        );
    }

    // 2. Center Ladder Frame: 112px tall, from y + 32.0 to y + 144.0
    let ly = y + 32.0;
    let h = 112.0;
    let light = is_light();
    let mut f_out = Path::new();
    f_out.rounded_rect(x, ly, w, h, 7.0);
    let (f_fill, f_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&f_out, &Paint::color(f_fill));
    let mut s_out = Paint::color(f_stroke);
    s_out.set_line_width(1.0);
    canvas.stroke_path(&f_out, &s_out);

    let mut f_tr = Path::new();
    f_tr.rounded_rect(x + 1.5, ly + 1.5, w - 3.0, h - 3.0, 5.5);
    canvas.fill_path(&f_tr, &Paint::color(if light {
        Color::rgb(0xB8, 0xB5, 0xAE)
    } else {
        Color::rgb(0x07, 0x07, 0x09)
    }));

    let mut f_lens = Path::new();
    f_lens.rounded_rect(x + 3.0, ly + 3.0, w - 6.0, h - 6.0, 4.5);
    let lens_grad = if light {
        Paint::linear_gradient(
            x,
            ly + 3.0,
            x,
            ly + h - 3.0,
            Color::rgb(0xEB, 0xD8, 0xE6),
            Color::rgb(0xDC, 0xC6, 0xDA),
        )
    } else {
        Paint::linear_gradient(
            x,
            ly + 3.0,
            x,
            ly + h - 3.0,
            Color::rgb(0x16, 0x06, 0x0E),
            Color::rgb(0x0D, 0x03, 0x08),
        )
    };
    canvas.fill_path(&f_lens, &lens_grad);
    let mut s_lens = Paint::color(if light {
        Color::rgba(0xFF, 0x00, 0x7F, 71)
    } else {
        Color::rgba(0xFF, 0x00, 0x7F, 56)
    });
    s_lens.set_line_width(0.8);
    canvas.stroke_path(&f_lens, &s_lens);

    let num_segs = 12usize;
    let top_pad = 7.0;
    let bot_pad = 7.0;
    let usable_h = h - top_pad - bot_pad;
    let step_y = usable_h / (num_segs as f32);
    let seg_h = step_y * 0.64;
    let bar_w = 16.0;
    let lx = x + 11.0;
    let rx = x + w - 11.0 - bar_w;

    let scale_ticks: [(usize, &str); 6] = [
        (0, "0"),
        (2, "2"),
        (4, "4"),
        (6, "6"),
        (8, "10"),
        (11, "14"),
    ];

    for i in 0..num_segs {
        let sy = ly + top_pad + (i as f32) * step_y;

        // 1. Output Signal (Left Column): lights from bottom UPWARDS
        let seg_from_bottom = num_segs - 1 - i;
        let mut seg_l = Path::new();
        seg_l.rounded_rect(lx, sy, bar_w, seg_h, 1.2);
        if seg_from_bottom < out_segs {
            canvas.fill_path(&seg_l, &Paint::color(dseg_active_pink()));
        } else {
            canvas.fill_path(&seg_l, &Paint::color(Color::rgba(0xFF, 0x00, 0x7F, 41)));
        }

        // 2. Gain Reduction (Right Column): lights from TOP DOWNWARDS (in amber/coral)
        let mut seg_r = Path::new();
        seg_r.rounded_rect(rx, sy, bar_w, seg_h, 1.2);
        if i < gr_segs {
            canvas.fill_path(&seg_r, &Paint::color(Color::rgb(0xFF, 0x55, 0x22)));
        } else {
            canvas.fill_path(&seg_r, &Paint::color(Color::rgba(0xFF, 0x55, 0x22, 35)));
        }
    }

    // Benchmark scale markings
    for &(idx, lbl) in &scale_ticks {
        let sy = ly + top_pad + (idx as f32) * step_y + seg_h * 0.5;
        draw_text_centered(
            canvas,
            cx,
            sy,
            lbl,
            fonts.bold_paint(
                if light {
                    Color::rgba(0x20, 0x22, 0x2A, 220)
                } else {
                    Color::rgba(0xEC, 0xE9, 0xE1, 140)
                },
                6.5,
            ),
        );
    }

    draw_text_centered(
        canvas,
        lx + bar_w * 0.5,
        ly + h - 4.5,
        "OUT",
        fonts.bold_paint(
            if light {
                Color::rgba(0x20, 0x22, 0x2A, 200)
            } else {
                silk_muted()
            },
            6.5,
        ),
    );
    draw_text_centered(
        canvas,
        rx + bar_w * 0.5,
        ly + h - 4.5,
        "GR",
        fonts.bold_paint(
            if light {
                Color::rgba(0x20, 0x22, 0x2A, 200)
            } else {
                silk_muted()
            },
            6.5,
        ),
    );

    // 3. Bottom Readout: GR or TRUE PK
    let bot_cy = ly + h + 6.0 + 13.0;
    if let Some(banner) = bot_banner {
        draw_dseg_display_left(
            canvas,
            cx,
            bot_cy,
            w,
            26.0,
            6,
            banner,
            7.5,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            cx,
            bot_cy,
            w,
            26.0,
            6,
            bot_tag,
            bot_str,
            7.5,
            fonts,
        );
    }
}

pub fn draw_clipper_meter<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    clip_l: f32,
    clip_r: f32,
    fonts: ThemeFonts,
) {
    let w = 26.0;
    let h = 58.0;
    let x = cx - w * 0.5;
    let y = cy - h * 0.5;

    draw_text_centered(
        canvas,
        cx,
        cy - 38.0,
        "CLIP",
        fonts.bold_paint(silk_muted(), 8.5),
    );

    let light = is_light();
    let mut outer = Path::new();
    outer.rounded_rect(x, y, w, h, 6.0);
    let (outer_fill, outer_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&outer, &Paint::color(outer_fill));
    let mut s_out = Paint::color(outer_stroke);
    s_out.set_line_width(1.0);
    canvas.stroke_path(&outer, &s_out);

    let mut f_tr = Path::new();
    f_tr.rounded_rect(x + 1.2, y + 1.2, w - 2.4, h - 2.4, 4.8);
    canvas.fill_path(&f_tr, &Paint::color(if light {
        Color::rgb(0xB8, 0xB5, 0xAE)
    } else {
        Color::rgb(0x07, 0x07, 0x09)
    }));

    // Dual-column LR 6-segment vertical LED ladder
    let num_segs = 6usize;
    let top_pad = 5.0;
    let bot_pad = 5.0;
    let usable_h = h - top_pad - bot_pad;
    let step_y = usable_h / (num_segs as f32);
    let seg_h = step_y * 0.65;
    let seg_w = 7.5;
    let lx = x + 4.0;
    let rx = x + w - 4.0 - seg_w;

    let thresholds = [0.2, 0.5, 1.0, 1.5, 2.0, 3.0];
    let lit_count_l = thresholds.iter().filter(|&&th| clip_l >= th).count();
    let lit_count_r = thresholds.iter().filter(|&&th| clip_r >= th).count();

    for i in 0..num_segs {
        let seg_from_bottom = num_segs - 1 - i;
        let sy = y + top_pad + (i as f32) * step_y;

        let col = if i == 0 {
            Color::rgb(0xFF, 0x22, 0x22)
        } else if i <= 2 {
            Color::rgb(0xFF, 0x66, 0x22)
        } else {
            dseg_active_pink()
        };

        for (bx, lit_count) in [(lx, lit_count_l), (rx, lit_count_r)] {
            let is_lit = seg_from_bottom < lit_count;
            let mut seg = Path::new();
            seg.rounded_rect(bx, sy, seg_w, seg_h, 1.0);
            if is_lit {
                canvas.fill_path(&seg, &Paint::color(col));
            } else {
                canvas.fill_path(&seg, &Paint::color(Color::rgba(0xFF, 0x00, 0x7F, 30)));
            }
        }
    }

    draw_text_centered(
        canvas,
        lx + seg_w * 0.5,
        cy + 31.5,
        "L",
        fonts.bold_paint(silk_muted(), 6.5),
    );
    draw_text_centered(
        canvas,
        rx + seg_w * 0.5,
        cy + 31.5,
        "R",
        fonts.bold_paint(silk_muted(), 6.5),
    );
}

pub fn draw_clipper_readout<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    clip_str: &str,
    banner: Option<&str>,
    fonts: ThemeFonts,
) {
    draw_text_centered(
        canvas,
        cx,
        cy - 38.0,
        "PEAK",
        fonts.bold_paint(silk_muted(), 8.5),
    );

    if let Some(banner) = banner {
        draw_dseg_display_left(
            canvas,
            cx,
            cy,
            44.0,
            26.0,
            3,
            banner,
            6.0,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            cx,
            cy,
            44.0,
            26.0,
            3,
            "",
            clip_str,
            6.0,
            fonts,
        );
    }

    draw_text_centered(
        canvas,
        cx,
        cy + 31.5,
        "dB",
        fonts.bold_paint(silk_muted(), 7.0),
    );
}

pub fn draw_lufs_display<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lufs_label: &str,
    lufs_tag: &str,
    lufs_str: &str,
    lufs_banner: Option<&str>,
    peak_str: &str,
    fonts: ThemeFonts,
) {
    let mut sub = Path::new();
    sub.rounded_rect(x, y, w, h, 10.0);
    let grad = Paint::linear_gradient(
        x,
        y,
        x,
        y + h,
        Color::rgb(0x11, 0x11, 0x14),
        Color::rgb(0x09, 0x09, 0x0B),
    );
    canvas.fill_path(&sub, &grad);
    let mut s1 = Paint::color(Color::rgb(0x26, 0x26, 0x2C));
    s1.set_line_width(1.2);
    canvas.stroke_path(&sub, &s1);

    draw_text_centered(
        canvas,
        x + w * 0.26,
        y + 16.0,
        lufs_label,
        fonts.bold_paint(silk_muted(), 9.0),
    );
    draw_text_centered(
        canvas,
        x + w * 0.74,
        y + 16.0,
        "TRUE PK",
        fonts.bold_paint(silk_muted(), 9.0),
    );

    if let Some(banner) = lufs_banner {
        draw_dseg_display_left(
            canvas,
            x + w * 0.26,
            y + 46.0,
            74.0,
            30.0,
            5,
            banner,
            7.0,
            fonts,
        );
    } else {
        draw_dseg_display_tagged(
            canvas,
            x + w * 0.26,
            y + 46.0,
            74.0,
            30.0,
            5,
            lufs_tag,
            lufs_str,
            7.0,
            fonts,
        );
    }
    draw_dseg_display_tagged(
        canvas,
        x + w * 0.74,
        y + 46.0,
        74.0,
        30.0,
        5,
        "",
        peak_str,
        7.0,
        fonts,
    );
}
