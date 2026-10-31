use nih_plug_vizia::vizia::vg::{Canvas, Color, LineCap, Paint, Path};
use std::f32::consts::PI;
use super::palette::is_light;

/// Unified hardware rack screw primitive with metallic washer, dome head,
/// angled cross drive slot, and full Dark Mode / Light Mode support.
///
/// Used across all rack chassis mounting positions, panel corners, and the
/// interactive bottom-right debug menu handle.
pub fn draw_screw_angled<T: nih_plug_vizia::vizia::vg::Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
    angle_deg: f32,
) {
    let light = is_light();
    let r_washer = 9.5;
    let r_inner_washer = 8.0;
    let r_head = 7.2;

    // 1. Subtle drop shadow under the outer washer
    let mut shadow = Path::new();
    shadow.circle(cx, cy + 1.2, r_washer + 1.2);
    let shadow_paint = Paint::radial_gradient(
        cx,
        cy + 1.2,
        r_washer - 1.5,
        r_washer + 2.0,
        if light {
            Color::rgba(0, 0, 0, 45)
        } else {
            Color::rgba(0, 0, 0, 160)
        },
        Color::rgba(0, 0, 0, 0),
    );
    canvas.fill_path(&shadow, &shadow_paint);

    // 2. Outer Washer Ring (machined chamfer)
    let mut washer = Path::new();
    washer.circle(cx, cy, r_washer);
    let washer_grad = if light {
        Paint::linear_gradient(
            cx - r_washer,
            cy - r_washer,
            cx + r_washer,
            cy + r_washer,
            Color::rgb(0xDD, 0xD9, 0xD1),
            Color::rgb(0xB4, 0xB0, 0xA8),
        )
    } else {
        Paint::linear_gradient(
            cx - r_washer,
            cy - r_washer,
            cx + r_washer,
            cy + r_washer,
            Color::rgb(0x3E, 0x42, 0x4C),
            Color::rgb(0x14, 0x16, 0x1A),
        )
    };
    canvas.fill_path(&washer, &washer_grad);
    let mut washer_stroke = Paint::color(if light {
        Color::rgb(0x8C, 0x88, 0x82)
    } else {
        Color::rgb(0x10, 0x11, 0x14)
    });
    washer_stroke.set_line_width(0.8);
    canvas.stroke_path(&washer, &washer_stroke);

    // Washer inner recess ring
    let mut w_inner = Path::new();
    w_inner.circle(cx, cy, r_inner_washer);
    let w_inner_grad = if light {
        Paint::linear_gradient(
            cx - r_inner_washer,
            cy - r_inner_washer,
            cx + r_inner_washer,
            cy + r_inner_washer,
            Color::rgb(0xA8, 0xA4, 0x9E),
            Color::rgb(0xCC, 0xC8, 0xC0),
        )
    } else {
        Paint::linear_gradient(
            cx - r_inner_washer,
            cy - r_inner_washer,
            cx + r_inner_washer,
            cy + r_inner_washer,
            Color::rgb(0x10, 0x11, 0x14),
            Color::rgb(0x28, 0x2A, 0x32),
        )
    };
    canvas.fill_path(&w_inner, &w_inner_grad);

    // 3. Metallic Screw Head Dome
    let mut dome = Path::new();
    dome.circle(cx, cy, r_head);
    let dome_grad = if light {
        Paint::linear_gradient(
            cx - r_head,
            cy - r_head,
            cx + r_head,
            cy + r_head,
            Color::rgb(0xFF, 0xFF, 0xFF),
            Color::rgb(0xC8, 0xC4, 0xBE),
        )
    } else {
        Paint::radial_gradient(
            cx - 1.2,
            cy - 1.2,
            0.5,
            r_head,
            Color::rgb(0x5A, 0x60, 0x6E),
            Color::rgb(0x1E, 0x20, 0x26),
        )
    };
    canvas.fill_path(&dome, &dome_grad);

    // Dome specular rim highlight
    let mut rim = Path::new();
    rim.circle(cx, cy, r_head);
    let mut rim_paint = Paint::linear_gradient(
        cx - r_head,
        cy - r_head,
        cx + r_head,
        cy + r_head,
        if light {
            Color::rgba(255, 255, 255, 230)
        } else {
            Color::rgba(255, 255, 255, 80)
        },
        if light {
            Color::rgba(0, 0, 0, 60)
        } else {
            Color::rgba(0, 0, 0, 180)
        },
    );
    rim_paint.set_line_width(0.7);
    canvas.stroke_path(&rim, &rim_paint);

    // 4. Cross Slot Drive (Phillips rotated by angle_deg)
    let rad = angle_deg * PI / 180.0;
    let arm_len = 4.2;
    let cos1 = rad.cos();
    let sin1 = rad.sin();
    let cos2 = (rad + std::f32::consts::FRAC_PI_2).cos();
    let sin2 = (rad + std::f32::consts::FRAC_PI_2).sin();

    let mut slot = Path::new();
    slot.move_to(cx - arm_len * cos1, cy - arm_len * sin1);
    slot.line_to(cx + arm_len * cos1, cy + arm_len * sin1);
    slot.move_to(cx - arm_len * cos2, cy - arm_len * sin2);
    slot.line_to(cx + arm_len * cos2, cy + arm_len * sin2);

    let mut slot_paint = Paint::color(if light {
        Color::rgb(0x4E, 0x4A, 0x44)
    } else {
        Color::rgb(0x0A, 0x0B, 0x0E)
    });
    slot_paint.set_line_width(1.5);
    slot_paint.set_line_cap(LineCap::Round);
    canvas.stroke_path(&slot, &slot_paint);

    // Specular highlight on edge of drive slot
    let mut hl_slot = Path::new();
    let h_off = 0.45;
    hl_slot.move_to(cx - arm_len * cos1 - h_off, cy - arm_len * sin1 - h_off);
    hl_slot.line_to(cx + arm_len * cos1 - h_off, cy + arm_len * sin1 - h_off);
    let mut hl_paint = Paint::color(if light {
        Color::rgba(255, 255, 255, 180)
    } else {
        Color::rgba(255, 255, 255, 50)
    });
    hl_paint.set_line_width(0.6);
    hl_paint.set_line_cap(LineCap::Round);
    canvas.stroke_path(&hl_slot, &hl_paint);
}

/// Standard flat screw convenience function (defaults to 18.5° authentic angle).
#[inline]
pub fn draw_screw<T: nih_plug_vizia::vizia::vg::Renderer>(
    canvas: &mut Canvas<T>,
    cx: f32,
    cy: f32,
) {
    draw_screw_angled(canvas, cx, cy, 18.5);
}

/// Backward compatibility struct for ScrewRenderer calls.
pub struct ScrewRenderer;

impl ScrewRenderer {
    #[inline]
    pub fn draw<T: nih_plug_vizia::vizia::vg::Renderer>(
        canvas: &mut Canvas<T>,
        cx: f32,
        cy: f32,
        angle_deg: f32,
    ) {
        draw_screw_angled(canvas, cx, cy, angle_deg);
    }

    /// Draws standard 4-corner rack screws for a module at the given width/height.
    pub fn draw_module_corners<T: nih_plug_vizia::vizia::vg::Renderer>(
        canvas: &mut Canvas<T>,
        width: f32,
        height: f32,
    ) {
        let margin_x = 15.0;
        let margin_y = 15.0;
        Self::draw(canvas, margin_x, margin_y, 22.0);
        Self::draw(canvas, width - margin_x, margin_y, 67.0);
        Self::draw(canvas, margin_x, height - margin_y, 114.0);
        Self::draw(canvas, width - margin_x, height - margin_y, 41.0);
    }
}
