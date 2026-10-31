//! Fourteen-segment readout primitives: cell tokenizers plus the smoked-glass display they
//! are drawn into.

#![allow(clippy::too_many_arguments)]

use nih_plug_vizia::vizia::vg::{Align, Baseline, Canvas, Color, Paint, Path, Renderer};

use super::palette::{dseg_active_pink, is_light};
use crate::gui::theme::ThemeFonts;

fn split_chars_and_dots(raw_str: &str) -> Vec<(char, bool)> {
    let mut items: Vec<(char, bool)> = Vec::new();
    for ch in raw_str.chars() {
        if ch == '.' {
            if let Some(last) = items.last_mut() {
                if !last.1 {
                    last.1 = true;
                    continue;
                }
            }
        }
        items.push((ch, false));
    }
    items
}

pub fn tokenize_dseg_cells(text_str: &str, num_cells: usize) -> Vec<(char, bool)> {
    let s_trim = text_str.trim().to_uppercase();
    let has_digit = s_trim.chars().any(|c| c.is_ascii_digit());

    if num_cells == 6 && s_trim.ends_with("DB") && has_digit {
        let num_part = s_trim[..s_trim.len() - 2].trim();
        let mut num_items = split_chars_and_dots(num_part);
        if num_items.len() < 4 {
            let mut padded = vec![(' ', false); 4 - num_items.len()];
            padded.append(&mut num_items);
            num_items = padded;
        } else if num_items.len() > 4 {
            num_items = num_items[num_items.len() - 4..].to_vec();
        }
        num_items.push(('D', false));
        num_items.push(('B', false));
        return num_items;
    }

    if num_cells == 6 && s_trim.ends_with("HZ") && has_digit {
        let freq_part = s_trim[..s_trim.len() - 2].trim();
        let mut freq_items = split_chars_and_dots(freq_part);
        if freq_items.len() < 4 {
            let mut padded = vec![(' ', false); 4 - freq_items.len()];
            padded.append(&mut freq_items);
            freq_items = padded;
        } else if freq_items.len() > 4 {
            freq_items = freq_items[freq_items.len() - 4..].to_vec();
        }
        freq_items.push(('H', false));
        freq_items.push(('Z', false));
        return freq_items;
    }

    let s = text_str.to_uppercase();
    let mut items = split_chars_and_dots(&s);
    if items.len() < num_cells {
        let pad_total = num_cells - items.len();
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;
        let mut res = vec![(' ', false); pad_left];
        res.append(&mut items);
        res.extend(vec![(' ', false); pad_right]);
        res
    } else {
        items.truncate(num_cells);
        items
    }
}

/// Fixed-field meter readout: a one-cell tag (`P`, `R`, `S`, `M`, `I`) hard-left and the numeric
/// value right-aligned in the remaining cells. Every character owns a fixed cell, so the decimal
/// point (which rides on the ones digit) and the tenths cell never move between values. An empty
/// tag lets the value use the full field, still right-aligned.
pub fn tokenize_dseg_tagged(tag: &str, value: &str, num_cells: usize) -> Vec<(char, bool)> {
    let tag_ch = tag.trim().chars().next();
    let field = if tag_ch.is_some() {
        num_cells.saturating_sub(1)
    } else {
        num_cells
    };

    let mut items = split_chars_and_dots(value.trim());
    if items.len() > field {
        items = items[items.len() - field..].to_vec();
    }

    let mut out = vec![(' ', false); num_cells];
    if let Some(ch) = tag_ch {
        out[0] = (ch.to_ascii_uppercase(), false);
    }
    let start = num_cells - items.len();
    for (i, item) in items.into_iter().enumerate() {
        out[start + i] = item;
    }
    out
}

/// Left-aligned banner text for momentary mode readouts (`PEAK`, `RMS`, `SHORT`, ...).
pub fn tokenize_dseg_left(text: &str, num_cells: usize) -> Vec<(char, bool)> {
    let mut items = split_chars_and_dots(text.trim());
    items.truncate(num_cells);
    items.resize(num_cells, (' ', false));
    items
}

pub fn draw_dseg_display<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    num_cells: usize,
    text_str: &str,
    rx_outer: f32,
    fonts: ThemeFonts,
) {
    let tokens = tokenize_dseg_cells(text_str, num_cells);
    draw_dseg_tokens(
        canvas, cx, cy, width, height, num_cells, &tokens, rx_outer, fonts,
    );
}

/// Tagged meter readout: `P`/`R`/`S`/`M`/`I` hard-left, numeric value right-aligned.
pub fn draw_dseg_display_tagged<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    num_cells: usize,
    tag: &str,
    value: &str,
    rx_outer: f32,
    fonts: ThemeFonts,
) {
    let tokens = tokenize_dseg_tagged(tag, value, num_cells);
    draw_dseg_tokens(
        canvas, cx, cy, width, height, num_cells, &tokens, rx_outer, fonts,
    );
}

/// Left-aligned momentary banner (`PEAK`, `RMS`, `SHORT`, ...).
pub fn draw_dseg_display_left<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    num_cells: usize,
    text_str: &str,
    rx_outer: f32,
    fonts: ThemeFonts,
) {
    let tokens = tokenize_dseg_left(text_str, num_cells);
    draw_dseg_tokens(
        canvas, cx, cy, width, height, num_cells, &tokens, rx_outer, fonts,
    );
}

pub fn draw_dseg_tokens<T: Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    width: f32,
    height: f32,
    num_cells: usize,
    tokens: &[(char, bool)],
    rx_outer: f32,
    fonts: ThemeFonts,
) {
    let left_x = cx - width * 0.5;
    let top_y = cy - height * 0.5;
    let rx_mid = (rx_outer - 1.5).max(2.0);
    let rx_lens = (rx_outer - 3.0).max(1.5);
    let rx_glare = (rx_outer - 4.0).max(1.0);

    let light = is_light();

    // 1. Outer Machined Readout Frame
    let mut f_out = Path::new();
    f_out.rounded_rect(left_x, top_y, width, height, rx_outer);
    let (f_fill, f_stroke) = if light {
        (Color::rgb(0xD0, 0xCD, 0xC6), Color::rgb(0xB5, 0xB2, 0xAA))
    } else {
        (Color::rgb(0x16, 0x16, 0x1B), Color::rgb(0x2C, 0x2C, 0x34))
    };
    canvas.fill_path(&f_out, &Paint::color(f_fill));
    let mut s_out = Paint::color(f_stroke);
    s_out.set_line_width(1.0);
    canvas.stroke_path(&f_out, &s_out);

    // 2. Deep Recessed Bezel Trench
    let mut f_tr = Path::new();
    f_tr.rounded_rect(left_x + 1.5, top_y + 1.5, width - 3.0, height - 3.0, rx_mid);
    let (tr_fill, tr_stroke) = if light {
        (Color::rgb(0xB8, 0xB5, 0xAE), Color::rgb(0x96, 0x93, 0x8C))
    } else {
        (Color::rgb(0x07, 0x07, 0x09), Color::rgb(0x00, 0x00, 0x00))
    };
    canvas.fill_path(&f_tr, &Paint::color(tr_fill));
    let mut s_tr = Paint::color(tr_stroke);
    s_tr.set_line_width(1.2);
    canvas.stroke_path(&f_tr, &s_tr);

    // 3. Smoked Tinted Glass Lens
    let mut f_lens = Path::new();
    f_lens.rounded_rect(
        left_x + 3.0,
        top_y + 3.0,
        width - 6.0,
        height - 6.0,
        rx_lens,
    );
    let lens_grad = if light {
        Paint::linear_gradient(
            left_x,
            top_y + 3.0,
            left_x,
            top_y + height - 3.0,
            Color::rgb(0xEB, 0xD8, 0xE6),
            Color::rgb(0xDC, 0xC6, 0xDA),
        )
    } else {
        Paint::linear_gradient(
            left_x,
            top_y + 3.0,
            left_x,
            top_y + height - 3.0,
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

    // 4. Optical Glass Top Glare Reflection
    let mut f_gl = Path::new();
    f_gl.rounded_rect(left_x + 4.0, top_y + 4.0, width - 8.0, 7.0, rx_glare);
    canvas.fill_path(&f_gl, &Paint::color(Color::rgba(255, 255, 255, if light { 50 } else { 13 })));

    // 5. Fixed 12.0px 14-Segment Starburst Grid + Active Characters
    let pitch = 12.0;
    let total_grid_w = (num_cells as f32 - 1.0) * pitch;
    let start_x = cx - total_grid_w * 0.5;
    let mid_y = cy + 0.5;

    let ghost_col = if light {
        Color::rgba(0xB8, 0x88, 0xA0, 76)
    } else {
        Color::rgba(0xFF, 0x00, 0x7F, 41)
    };
    let mut ghost_paint = fonts.dseg_paint(ghost_col, 13.8);
    ghost_paint.set_text_align(Align::Center);
    ghost_paint.set_text_baseline(Baseline::Middle);

    let bloom_col = if light {
        Color::rgba(0xFF, 0x00, 0x7F, 50)
    } else {
        Color::rgba(0xFF, 0x00, 0x7F, 85)
    };
    let mut bloom_paint = fonts.dseg_paint(bloom_col, 14.3);
    bloom_paint.set_text_align(Align::Center);
    bloom_paint.set_text_baseline(Baseline::Middle);

    let mut active_paint = fonts.dseg_paint(dseg_active_pink(), 13.8);
    active_paint.set_text_align(Align::Center);
    active_paint.set_text_baseline(Baseline::Middle);

    let mut buf = [0u8; 4];
    for i in 0..num_cells {
        let cell_x = start_x + (i as f32) * pitch;
        let _ = canvas.fill_text(cell_x, mid_y, "~", &ghost_paint);
        let Some(&(ch, has_dot)) = tokens.get(i) else {
            continue;
        };
        if ch != ' ' {
            let s: &str = ch.encode_utf8(&mut buf);
            let _ = canvas.fill_text(cell_x, mid_y, s, &bloom_paint);
            let _ = canvas.fill_text(cell_x, mid_y, s, &active_paint);
        }
        if has_dot {
            let mut dot = Path::new();
            dot.circle(cell_x + 5.4, mid_y + 6.2, 1.15);
            canvas.fill_path(&dot, &Paint::color(dseg_active_pink()));
        }
    }
}
