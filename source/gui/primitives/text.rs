//! Text helpers. Both align vertically to the middle so coordinates line up with the
//! engraved hardware legends.

use nih_plug_vizia::vizia::vg::{Align, Baseline, Canvas, Paint, Renderer};

pub fn draw_text_centered<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    text: &str,
    mut paint: Paint,
) {
    paint.set_text_align(Align::Center);
    paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x, y, text, &paint);
}

pub fn draw_text<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    text: &str,
    mut paint: Paint,
) {
    paint.set_text_align(Align::Left);
    paint.set_text_baseline(Baseline::Middle);
    let _ = canvas.fill_text(x, y, text, &paint);
}
