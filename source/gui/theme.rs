use nih_plug_vizia::vizia::vg::{Color, FontId, Paint};

#[derive(Copy, Clone, Debug)]
pub struct ThemeFonts {
    pub regular: FontId,
    pub bold: FontId,
    pub dseg: FontId,
}

impl ThemeFonts {
    #[inline]
    pub fn regular_paint(&self, color: Color, size: f32) -> Paint {
        let mut paint = Paint::color(color);
        paint.set_font(&[self.regular]);
        paint.set_font_size(size);
        paint
    }

    #[inline]
    pub fn bold_paint(&self, color: Color, size: f32) -> Paint {
        let mut paint = Paint::color(color);
        paint.set_font(&[self.bold]);
        paint.set_font_size(size);
        paint
    }

    #[inline]
    pub fn dseg_paint(&self, color: Color, size: f32) -> Paint {
        let mut paint = Paint::color(color);
        paint.set_font(&[self.dseg]);
        paint.set_font_size(size);
        paint
    }
}

pub struct Theme;

impl Theme {
    // Chassis & Panel Colors
    #[inline]
    pub fn panel_bg_top() -> Color {
        Color::rgb(0x24, 0x27, 0x2b)
    }
    #[inline]
    pub fn panel_bg_mid() -> Color {
        Color::rgb(0x1c, 0x1e, 0x22)
    }
    #[inline]
    pub fn panel_bg_bot() -> Color {
        Color::rgb(0x16, 0x18, 0x1b)
    }
    #[inline]
    pub fn panel_inner_rim() -> Color {
        Color::rgba(0xff, 0xff, 0xff, 18)
    } // ~0.07 opacity

    // Anodized Aluminum Tint Colors for Paint::image_tint
    // Modulate these values to achieve any custom metallic anodized finish:
    #[inline]
    pub fn panel_tint() -> Color {
        Color::rgb(0x40, 0x44, 0x4b)
    } // Dark graphite faceplate
    #[inline]
    pub fn poti_skirt_tint() -> Color {
        Color::rgb(0x48, 0x4d, 0x55)
    } // Anodized knob skirt
    #[inline]
    pub fn poti_cap_tint() -> Color {
        Color::rgb(0x5c, 0x62, 0x6b)
    } // Spun metal knob cap
    #[inline]
    pub fn poti_small_skirt_tint() -> Color {
        Color::rgb(0x3e, 0x42, 0x48)
    }
    #[inline]
    pub fn poti_small_cap_tint() -> Color {
        Color::rgb(0x50, 0x55, 0x5d)
    }

    // Active Neon Green & Glow
    #[inline]
    pub fn neon_green() -> Color {
        Color::rgb(0x00, 0xff, 0x66)
    }
    #[inline]
    pub fn neon_green_glow() -> Color {
        Color::rgba(0x00, 0xff, 0x66, 100)
    }

    // Silk / Hardware Off-White
    #[inline]
    pub fn silk_white() -> Color {
        Color::rgb(0xec, 0xe9, 0xe1)
    }
    #[inline]
    pub fn silk_muted() -> Color {
        Color::rgba(0xec, 0xe9, 0xe1, 140)
    } // ~0.55 opacity
    #[inline]
    pub fn silk_dim() -> Color {
        Color::rgba(0xec, 0xe9, 0xe1, 95)
    } // ~0.37 opacity

    // Frequency Arc Blue
    #[inline]
    pub fn freq_blue() -> Color {
        Color::rgb(0x86, 0xa2, 0xbe)
    }
    #[inline]
    pub fn freq_blue_text() -> Color {
        Color::rgba(0x86, 0xa2, 0xbe, 191)
    } // ~0.75 opacity
    #[inline]
    pub fn freq_blue_dot() -> Color {
        Color::rgba(0x86, 0xa2, 0xbe, 102)
    } // ~0.40 opacity
    #[inline]
    pub fn freq_blue_arc() -> Color {
        Color::rgba(0x86, 0xa2, 0xbe, 38)
    } // ~0.15 opacity

    // Monospace Readout Amber
    #[inline]
    pub fn readout_amber() -> Color {
        Color::rgba(0xff, 0xb3, 0x47, 230)
    } // ~0.90 opacity

    // Knobs (Poti)
    #[inline]
    pub fn poti_ring_top() -> Color {
        Color::rgb(0x9a, 0xa0, 0xa8)
    }
    #[inline]
    pub fn poti_ring_mid() -> Color {
        Color::rgb(0x5d, 0x62, 0x69)
    }
    #[inline]
    pub fn poti_ring_bot() -> Color {
        Color::rgb(0x2b, 0x2e, 0x32)
    }

    #[inline]
    pub fn poti_skirt_top() -> Color {
        Color::rgb(0x4a, 0x4e, 0x54)
    }
    #[inline]
    pub fn poti_skirt_mid() -> Color {
        Color::rgb(0x26, 0x28, 0x2c)
    }
    #[inline]
    pub fn poti_skirt_bot() -> Color {
        Color::rgb(0x11, 0x12, 0x14)
    }

    #[inline]
    pub fn poti_cap_top() -> Color {
        Color::rgb(0x5a, 0x5f, 0x66)
    }
    #[inline]
    pub fn poti_cap_mid() -> Color {
        Color::rgb(0x34, 0x37, 0x3c)
    }
    #[inline]
    pub fn poti_cap_bot() -> Color {
        Color::rgb(0x1c, 0x1e, 0x21)
    }

    #[inline]
    pub fn poti_rim_light() -> Color {
        Color::rgba(0x8b, 0x90, 0x98, 230)
    }
    #[inline]
    pub fn poti_rim_dark() -> Color {
        Color::rgba(0x05, 0x05, 0x05, 230)
    }

    // Model Selector Pill
    #[inline]
    pub fn pill_bg() -> Color {
        Color::rgb(0x10, 0x11, 0x13)
    }
    #[inline]
    pub fn pill_active_thumb() -> Color {
        Color::rgb(0x2c, 0x2f, 0x34)
    }

    // Separators & Shadows
    #[inline]
    pub fn separator_line() -> Color {
        Color::rgba(0x00, 0x00, 0x00, 115)
    } // ~0.45 opacity
    #[inline]
    pub fn drop_shadow() -> Color {
        Color::rgba(0x00, 0x00, 0x00, 166)
    } // ~0.65 opacity

    // Dimensions
    pub const RACK_WIDTH: f32 = 940.0;
    pub const RACK_HEIGHT: f32 = 270.0;
    pub const RACK_RADIUS: f32 = 10.0;
}
