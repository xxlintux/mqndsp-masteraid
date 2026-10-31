//! Panel card and plate-well primitives: the recessed chassis surfaces the controls sit in.

use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer};
use super::palette::is_light;

pub fn draw_card<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    has_header_divider: bool,
) {
    let light = is_light();

    // Drop shadow
    let mut sh = Path::new();
    sh.rounded_rect(x - 4.0, y + 2.0, w + 8.0, h + 12.0, 18.0);
    let sh_paint = Paint::box_gradient(
        x,
        y + 8.0,
        w,
        h,
        14.0,
        16.0,
        if light {
            Color::rgba(0, 0, 0, 80)
        } else {
            Color::rgba(0, 0, 0, 185)
        },
        Color::rgba(0, 0, 0, 0),
    );
    canvas.fill_path(&sh, &sh_paint);

    // Main card body
    let mut body = Path::new();
    body.rounded_rect(x, y, w, h, 14.0);
    let body_grad = if light {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0xEA, 0xE7, 0xE2),
            Color::rgb(0xD8, 0xD4, 0xCD),
        )
    } else {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0x14, 0x14, 0x16),
            Color::rgb(0x0B, 0x0B, 0x0D),
        )
    };
    canvas.fill_path(&body, &body_grad);

    let mut border = Paint::color(if light {
        Color::rgb(0xB4, 0xB0, 0xA8)
    } else {
        Color::rgb(0x26, 0x26, 0x2C)
    });
    border.set_line_width(1.2);
    canvas.stroke_path(&body, &border);

    let mut inner = Path::new();
    inner.rounded_rect(x + 1.2, y + 1.2, w - 2.4, h - 2.4, 12.8);
    let mut inner_stroke = Paint::color(if light {
        Color::rgba(0xFF, 0xFF, 0xFF, 180)
    } else {
        Color::rgba(0x38, 0x38, 0x40, 115)
    });
    inner_stroke.set_line_width(0.8);
    canvas.stroke_path(&inner, &inner_stroke);

    if has_header_divider {
        let mut div1 = Path::new();
        div1.move_to(x + 1.0, y + 38.0);
        div1.line_to(x + w - 1.0, y + 38.0);
        let mut p1 = Paint::color(if light {
            Color::rgba(0x90, 0x8C, 0x86, 160)
        } else {
            Color::rgba(0, 0, 0, 180)
        });
        p1.set_line_width(1.0);
        canvas.stroke_path(&div1, &p1);

        let mut div2 = Path::new();
        div2.move_to(x + 1.0, y + 39.0);
        div2.line_to(x + w - 1.0, y + 39.0);
        let mut p2 = Paint::color(if light {
            Color::rgba(0xFF, 0xFF, 0xFF, 200)
        } else {
            Color::rgba(0x38, 0x38, 0x40, 90)
        });
        p2.set_line_width(1.0);
        canvas.stroke_path(&div2, &p2);
    }
}

pub fn draw_plate_well_160x200<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    let light = is_light();
    let mut body = Path::new();
    body.rounded_rect(x, y, w, h, 16.0);
    let grad = if light {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0xE0, 0xDC, 0xD5),
            Color::rgb(0xCC, 0xC8, 0xC0),
        )
    } else {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + h,
            Color::rgb(0x14, 0x14, 0x16),
            Color::rgb(0x0B, 0x0B, 0x0D),
        )
    };
    canvas.fill_path(&body, &grad);

    let mut b1 = Paint::color(if light {
        Color::rgb(0xB0, 0xAC, 0xA4)
    } else {
        Color::rgb(0x26, 0x26, 0x2C)
    });
    b1.set_line_width(1.0);
    canvas.stroke_path(&body, &b1);

    let mut inner = Path::new();
    inner.rounded_rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, 15.0);
    let mut b2 = Paint::color(if light {
        Color::rgba(0xFF, 0xFF, 0xFF, 160)
    } else {
        Color::rgba(0x38, 0x38, 0x40, 115)
    });
    b2.set_line_width(0.7);
    canvas.stroke_path(&inner, &b2);
}

pub fn draw_plate_well_80x100<T: Renderer>(canvas: &mut Canvas<T>, x: f32, y: f32) {
    let light = is_light();
    let mut body = Path::new();
    body.rounded_rect(x, y, 80.0, 100.0, 12.0);
    let grad = if light {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + 100.0,
            Color::rgb(0xE0, 0xDC, 0xD5),
            Color::rgb(0xCC, 0xC8, 0xC0),
        )
    } else {
        Paint::linear_gradient(
            x,
            y,
            x,
            y + 100.0,
            Color::rgb(0x14, 0x14, 0x16),
            Color::rgb(0x0B, 0x0B, 0x0D),
        )
    };
    canvas.fill_path(&body, &grad);

    let mut b1 = Paint::color(if light {
        Color::rgb(0xB0, 0xAC, 0xA4)
    } else {
        Color::rgb(0x26, 0x26, 0x2C)
    });
    b1.set_line_width(1.0);
    canvas.stroke_path(&body, &b1);

    let mut inner = Path::new();
    inner.rounded_rect(x + 1.0, y + 1.0, 78.0, 98.0, 11.0);
    let mut b2 = Paint::color(if light {
        Color::rgba(0xFF, 0xFF, 0xFF, 160)
    } else {
        Color::rgba(0x38, 0x38, 0x40, 115)
    });
    b2.set_line_width(0.7);
    canvas.stroke_path(&inner, &b2);
}
