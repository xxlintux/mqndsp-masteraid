//! Official MQN DSP vector logo primitive.

use nih_plug_vizia::vizia::vg::{Canvas, Color, Paint, Path, Renderer, Solidity};

pub fn draw_mqn_logo<T: Renderer>(
    canvas: &mut Canvas<T>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) {
    let sx = w / 882.7;
    let sy = h / 256.9;
    let mut p = Path::new();

    // Subpath 0 (outer)
    p.move_to(x + 864.93 * sx, y + 0.00 * sy);
    p.line_to(x + 865.41 * sx, y + 0.42 * sy);
    p.line_to(x + 864.23 * sx, y + 3.42 * sy);
    p.line_to(x + 788.56 * sx, y + 171.42 * sy);
    p.line_to(x + 759.20 * sx, y + 238.42 * sy);
    p.line_to(x + 758.03 * sx, y + 240.42 * sy);
    p.line_to(x + 742.93 * sx, y + 256.44 * sy);
    p.line_to(x + 742.63 * sx, y + 255.42 * sy);
    p.line_to(x + 756.67 * sx, y + 223.42 * sy);
    p.line_to(x + 849.75 * sx, y + 15.42 * sy);
    p.line_to(x + 852.62 * sx, y + 11.42 * sy);
    p.line_to(x + 864.93 * sx, y + 0.00 * sy);
    p.close();

    // Subpath 1 (outer)
    p.move_to(x + 830.93 * sx, y + 15.03 * sy);
    p.line_to(x + 831.42 * sx, y + 15.42 * sy);
    p.line_to(x + 831.08 * sx, y + 16.42 * sy);
    p.line_to(x + 827.32 * sx, y + 24.42 * sy);
    p.line_to(x + 763.51 * sx, y + 153.42 * sy);
    p.line_to(x + 740.93 * sx, y + 200.97 * sy);
    p.line_to(x + 641.93 * sx, y + 101.98 * sy);
    p.line_to(x + 640.93 * sx, y + 102.49 * sy);
    p.line_to(x + 625.74 * sx, y + 132.42 * sy);
    p.line_to(x + 600.34 * sx, y + 184.42 * sy);
    p.line_to(x + 586.93 * sx, y + 184.82 * sy);
    p.line_to(x + 552.90 * sx, y + 184.42 * sy);
    p.line_to(x + 627.93 * sx, y + 28.08 * sy);
    p.line_to(x + 725.93 * sx, y + 126.26 * sy);
    p.line_to(x + 730.19 * sx, y + 119.42 * sy);
    p.line_to(x + 768.66 * sx, y + 43.42 * sy);
    p.line_to(x + 830.93 * sx, y + 15.03 * sy);
    p.close();

    // Subpath 2 (outer)
    p.move_to(x + 0.93 * sx, y + 32.03 * sy);
    p.line_to(x + 177.93 * sx, y + 32.09 * sy);
    p.line_to(x + 234.93 * sx, y + 91.99 * sy);
    p.line_to(x + 235.93 * sx, y + 91.97 * sy);
    p.line_to(x + 246.93 * sx, y + 84.77 * sy);
    p.line_to(x + 325.93 * sx, y + 32.00 * sy);
    p.line_to(x + 377.93 * sx, y + 31.97 * sy);
    p.line_to(x + 380.12 * sx, y + 32.42 * sy);
    p.line_to(x + 333.47 * sx, y + 124.42 * sy);
    p.line_to(x + 312.69 * sx, y + 167.42 * sy);
    p.line_to(x + 248.93 * sx, y + 207.59 * sy);
    p.line_to(x + 246.93 * sx, y + 208.63 * sy);
    p.line_to(x + 247.44 * sx, y + 206.42 * sy);
    p.line_to(x + 298.03 * sx, y + 97.42 * sy);
    p.line_to(x + 292.93 * sx, y + 100.21 * sy);
    p.line_to(x + 264.93 * sx, y + 118.82 * sy);
    p.line_to(x + 220.93 * sx, y + 146.78 * sy);
    p.line_to(x + 173.93 * sx, y + 95.04 * sy);
    p.line_to(x + 172.45 * sx, y + 96.42 * sy);
    p.line_to(x + 165.78 * sx, y + 107.42 * sy);
    p.line_to(x + 137.09 * sx, y + 156.42 * sy);
    p.line_to(x + 124.16 * sx, y + 172.42 * sy);
    p.line_to(x + 110.93 * sx, y + 183.75 * sy);
    p.line_to(x + 26.93 * sx, y + 247.74 * sy);
    p.line_to(x + 26.61 * sx, y + 247.42 * sy);
    p.line_to(x + 29.68 * sx, y + 242.42 * sy);
    p.line_to(x + 120.27 * sx, y + 96.42 * sy);
    p.line_to(x + 132.27 * sx, y + 76.42 * sy);
    p.line_to(x + 134.09 * sx, y + 72.42 * sy);
    p.line_to(x + 36.93 * sx, y + 43.89 * sy);
    p.line_to(x + 0.93 * sx, y + 32.82 * sy);
    p.line_to(x + 0.00 * sx, y + 32.42 * sy);
    p.line_to(x + 0.93 * sx, y + 32.03 * sy);
    p.close();

    // Subpath 3 (outer)
    p.move_to(x + 405.93 * sx, y + 31.94 * sy);
    p.line_to(x + 569.93 * sx, y + 31.98 * sy);
    p.line_to(x + 595.20 * sx, y + 59.42 * sy);
    p.line_to(x + 595.95 * sx, y + 60.42 * sy);
    p.line_to(x + 595.40 * sx, y + 62.42 * sy);
    p.line_to(x + 561.66 * sx, y + 128.42 * sy);
    p.line_to(x + 539.45 * sx, y + 173.42 * sy);
    p.line_to(x + 529.65 * sx, y + 182.42 * sy);
    p.line_to(x + 529.63 * sx, y + 183.42 * sy);
    p.line_to(x + 558.19 * sx, y + 222.42 * sy);
    p.line_to(x + 582.25 * sx, y + 256.42 * sy);
    p.line_to(x + 581.93 * sx, y + 256.91 * sy);
    p.line_to(x + 488.31 * sx, y + 183.42 * sy);
    p.line_to(x + 432.61 * sx, y + 129.42 * sy);
    p.line_to(x + 433.93 * sx, y + 128.87 * sy);
    p.line_to(x + 491.93 * sx, y + 128.85 * sy);
    p.line_to(x + 492.93 * sx, y + 129.11 * sy);
    p.line_to(x + 503.93 * sx, y + 140.92 * sy);
    p.line_to(x + 504.93 * sx, y + 140.10 * sy);
    p.line_to(x + 509.11 * sx, y + 132.42 * sy);
    p.line_to(x + 539.13 * sx, y + 74.42 * sy);
    p.line_to(x + 539.99 * sx, y + 72.42 * sy);
    p.line_to(x + 538.93 * sx, y + 71.89 * sy);
    p.line_to(x + 421.93 * sx, y + 71.90 * sy);
    p.line_to(x + 420.93 * sx, y + 71.93 * sy);
    p.line_to(x + 419.58 * sx, y + 73.42 * sy);
    p.line_to(x + 383.03 * sx, y + 144.42 * sy);
    p.line_to(x + 383.75 * sx, y + 145.42 * sy);
    p.line_to(x + 384.93 * sx, y + 145.51 * sy);
    p.line_to(x + 437.93 * sx, y + 145.64 * sy);
    p.line_to(x + 474.74 * sx, y + 181.42 * sy);
    p.line_to(x + 477.47 * sx, y + 184.42 * sy);
    p.line_to(x + 476.93 * sx, y + 184.79 * sy);
    p.line_to(x + 350.93 * sx, y + 184.82 * sy);
    p.line_to(x + 331.81 * sx, y + 164.42 * sy);
    p.line_to(x + 330.50 * sx, y + 162.42 * sy);
    p.line_to(x + 330.85 * sx, y + 161.42 * sy);
    p.line_to(x + 386.46 * sx, y + 46.42 * sy);
    p.line_to(x + 405.93 * sx, y + 31.94 * sy);
    p.close();

    // Subpath 4 (outer)
    p.move_to(x + 864.93 * sx, y + 32.41 * sy);
    p.line_to(x + 865.93 * sx, y + 32.20 * sy);
    p.line_to(x + 867.07 * sx, y + 33.42 * sy);
    p.line_to(x + 880.93 * sx, y + 49.60 * sy);
    p.line_to(x + 882.73 * sx, y + 61.42 * sy);
    p.line_to(x + 880.19 * sx, y + 68.42 * sy);
    p.line_to(x + 869.42 * sx, y + 92.42 * sy);
    p.line_to(x + 869.93 * sx, y + 94.32 * sy);
    p.line_to(x + 878.31 * sx, y + 104.42 * sy);
    p.line_to(x + 878.09 * sx, y + 106.42 * sy);
    p.line_to(x + 875.03 * sx, y + 113.42 * sy);
    p.line_to(x + 873.93 * sx, y + 115.63 * sy);
    p.line_to(x + 872.93 * sx, y + 115.92 * sy);
    p.line_to(x + 843.64 * sx, y + 81.42 * sy);
    p.line_to(x + 849.68 * sx, y + 66.42 * sy);
    p.line_to(x + 864.93 * sx, y + 32.41 * sy);
    p.close();

    // Subpath 5 (hole)
    p.move_to(x + 867.93 * sx, y + 53.59 * sy);
    p.line_to(x + 856.75 * sx, y + 78.42 * sy);
    p.line_to(x + 860.93 * sx, y + 83.25 * sy);
    p.line_to(x + 872.19 * sx, y + 58.42 * sy);
    p.line_to(x + 869.31 * sx, y + 54.42 * sy);
    p.line_to(x + 867.93 * sx, y + 53.59 * sy);
    p.close();
    p.solidity(Solidity::Hole);

    // Subpath 6 (outer)
    p.move_to(x + 840.93 * sx, y + 87.15 * sy);
    p.line_to(x + 842.28 * sx, y + 92.42 * sy);
    p.line_to(x + 844.04 * sx, y + 105.42 * sy);
    p.line_to(x + 833.48 * sx, y + 129.42 * sy);
    p.line_to(x + 832.40 * sx, y + 132.42 * sy);
    p.line_to(x + 832.73 * sx, y + 133.42 * sy);
    p.line_to(x + 834.93 * sx, y + 136.04 * sy);
    p.line_to(x + 835.93 * sx, y + 135.43 * sy);
    p.line_to(x + 847.53 * sx, y + 109.42 * sy);
    p.line_to(x + 848.93 * sx, y + 107.98 * sy);
    p.line_to(x + 852.93 * sx, y + 107.09 * sy);
    p.line_to(x + 855.93 * sx, y + 107.24 * sy);
    p.line_to(x + 866.17 * sx, y + 119.42 * sy);
    p.line_to(x + 867.57 * sx, y + 128.42 * sy);
    p.line_to(x + 864.21 * sx, y + 137.42 * sy);
    p.line_to(x + 847.93 * sx, y + 172.96 * sy);
    p.line_to(x + 845.06 * sx, y + 154.42 * sy);
    p.line_to(x + 849.68 * sx, y + 142.42 * sy);
    p.line_to(x + 856.02 * sx, y + 128.42 * sy);
    p.line_to(x + 856.36 * sx, y + 126.42 * sy);
    p.line_to(x + 853.93 * sx, y + 123.55 * sy);
    p.line_to(x + 852.93 * sx, y + 123.87 * sy);
    p.line_to(x + 840.93 * sx, y + 150.36 * sy);
    p.line_to(x + 836.93 * sx, y + 151.77 * sy);
    p.line_to(x + 832.93 * sx, y + 151.83 * sy);
    p.line_to(x + 823.29 * sx, y + 140.42 * sy);
    p.line_to(x + 821.56 * sx, y + 130.42 * sy);
    p.line_to(x + 826.47 * sx, y + 118.42 * sy);
    p.line_to(x + 840.93 * sx, y + 87.15 * sy);
    p.close();

    // Subpath 7 (outer)
    p.move_to(x + 812.93 * sx, y + 149.88 * sy);
    p.line_to(x + 821.93 * sx, y + 149.94 * sy);
    p.line_to(x + 823.41 * sx, y + 151.42 * sy);
    p.line_to(x + 840.18 * sx, y + 171.42 * sy);
    p.line_to(x + 842.38 * sx, y + 183.42 * sy);
    p.line_to(x + 842.47 * sx, y + 186.42 * sy);
    p.line_to(x + 823.93 * sx, y + 229.59 * sy);
    p.line_to(x + 819.60 * sx, y + 225.42 * sy);
    p.line_to(x + 792.96 * sx, y + 194.42 * sy);
    p.line_to(x + 812.93 * sx, y + 149.88 * sy);
    p.close();

    // Subpath 8 (hole)
    p.move_to(x + 818.93 * sx, y + 165.39 * sy);
    p.line_to(x + 817.93 * sx, y + 165.51 * sy);
    p.line_to(x + 815.52 * sx, y + 170.42 * sy);
    p.line_to(x + 806.43 * sx, y + 191.42 * sy);
    p.line_to(x + 815.60 * sx, y + 202.42 * sy);
    p.line_to(x + 820.93 * sx, y + 207.84 * sy);
    p.line_to(x + 831.94 * sx, y + 182.42 * sy);
    p.line_to(x + 831.11 * sx, y + 179.42 * sy);
    p.line_to(x + 818.93 * sx, y + 165.39 * sy);
    p.close();
    p.solidity(Solidity::Hole);

    canvas.fill_path(&p, &Paint::color(color));
}