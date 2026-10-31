"""
Masteraid Modular Console Composer (MQN Tactile Suite Edition)
Composes the full 1920x1080 Masteraid Mastering Console using the official MQN Tactile Design System:
- Primary Accent: Pure Neon Magenta (#FF007F)
- Universal Metallic Plate Well Gradients (#141416 to #0B0B0D)
- Embedded DSEG14 Classic Italic 14-Segment Display Font (native slant, zero faux-oblique)
- 4x MQN Dual Concentric Knobs + 6-Cell Readouts (Mastering Parametric EQ, static band label, momentary value display, zero static values on pots)
- 3x MQN Single Big Knobs + 6-Cell Readouts (Input Trim, Comp Threshold, Master Output, static label, momentary value display, zero static values on pots)
- 19x MQN 6-Flute Scalloped Small Pots with Concentric Circular Base Collars & 11-Tick Dials (-135..+135 deg in 27 deg steps)
- 2x MQN Calibrated Analog VU Meters (-20 to +3 VU, 9 tick values, #FF007F overload zone)
"""

import base64
import json
import os

from components.knobs import (
    ACCENT_MAGENTA,
    draw_6cell_readout,
    draw_knob_concentric,
    draw_knob_large,
    draw_knob_small,
    get_knob_defs,
)
from components.buttons import (
    draw_model_select,
    draw_vintage_bypass_button,
    draw_vintage_push_button,
    get_button_css,
    get_button_defs,
)
from components.meters import (
    draw_input_ladder,
    draw_lufs_display,
    draw_segmented_selector,
    draw_vu_meter,
    get_meter_defs,
)

THEME_FILE = os.path.join(os.path.dirname(__file__), "..", "assets", "gui", "theme_config.json")
FONT_FILE = os.path.join(os.path.dirname(__file__), "..", "assets", "fonts", "dseg14-classic-italic.woff2")

with open(FONT_FILE, "rb") as _f:
    FONT_B64 = base64.b64encode(_f.read()).decode("ascii")

DEFAULT_THEME = {
    "brushed_freq_x": 0.003,
    "brushed_freq_y": 0.85,
    "brushed_chassis_opacity": 0.14,
    "brushed_card_opacity": 0.12,
    "panel_top": "#16161A",
    "panel_bottom": "#0C0C0E",
    "led_green": "#FF007F",
    "amber_glow": "#FF007F",
    "freq_blue": "#8A8A92",
    "silk_text": "#ECE9E1",
    "vu_parchment_top": "#2A2A2F",
    "vu_parchment_bottom": "#1E1E22",
    "knob_tone": 0.0,
    "specular_light": 0.80,
    "card_shadow_opacity": 0.75,
}


def load_theme():
    theme = dict(DEFAULT_THEME)
    if os.path.exists(THEME_FILE):
        try:
            with open(THEME_FILE, "r", encoding="utf-8") as f:
                saved = json.load(f)
                theme.update(saved)
        except Exception:
            pass
    return theme


WIDTH = 1360
HEIGHT = 952


def draw_screw(cx, cy):
    return f'''<g transform="translate({cx},{cy})">
      <circle r="9.5" fill="#08080A" stroke="#26262C" stroke-width="0.8"/>
      <circle r="8.0" fill="url(#screw-grad)" stroke="#141417" stroke-width="0.8"/>
      <line x1="-4.5" y1="-1.5" x2="4.5" y2="1.5" stroke="#101013" stroke-width="1.5" stroke-linecap="round"/>
      <line x1="-1.5" y1="4.5" x2="1.5" y2="-4.5" stroke="#101013" stroke-width="1.5" stroke-linecap="round"/>
    </g>'''


def draw_card(id_name, x, y, w, h, has_header_divider=True):
    """Draws an MQN rack module card using the universal metallic plate well gradient."""
    div_svg = ""
    if has_header_divider:
        div_svg = f'''
      <line x1="{x+1}" y1="{y+38}" x2="{x+w-1}" y2="{y+38}" stroke="#000000" stroke-opacity="0.7"/>
      <line x1="{x+1}" y1="{y+39}" x2="{x+w-1}" y2="{y+39}" stroke="#383840" stroke-opacity="0.35"/>'''
    return f'''<!-- MQN MODULE CARD: {id_name} -->
    <g id="card_{id_name}">
      <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="14" fill="url(#mqn_plate_well_grad)" stroke="#26262C" stroke-width="1.2" filter="url(#shadow-card)"/>
      <rect x="{x+1.2}" y="{y+1.2}" width="{w-2.4}" height="{h-2.4}" rx="12.8" fill="none" stroke="#383840" stroke-width="0.8" stroke-opacity="0.45"/>{div_svg}
    </g>'''


def draw_coloration_cassette(cassette_id, x, y, w, h, amp_param, amp_alias, trans_param, trans_alias, push_param, is_tall=False):
    """MQN Coloration Cassette with balanced breathing room, 9-cell 14-segment steppers, and squared smoked-glass push button."""
    res = [f'<!-- MQN COLORATION CASSETTE: {cassette_id} -->']
    res.append(f'<g id="{cassette_id}">')
    res.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="12" fill="url(#mqn_subpanel_grad)" stroke="#26262C" stroke-width="1.2"/>')
    res.append(f'<rect x="{x+1}" y="{y+1}" width="{w-2}" height="{h-2}" rx="11" fill="none" stroke="#383840" stroke-width="0.7" stroke-opacity="0.35"/>')

    # 4 Corner slotted screws
    screws = [(x + 10, y + 10), (x + w - 10, y + 10), (x + 10, y + h - 10), (x + w - 10, y + h - 10)]
    for sx, sy in screws:
        res.append(f'''<g transform="translate({sx}, {sy})">
          <circle cx="0" cy="0" r="3.2" fill="url(#screw-grad)" stroke="#111" stroke-width="0.6"/>
          <line x1="-2.2" y1="0" x2="2.2" y2="0" stroke="#111" stroke-width="0.8"/>
        </g>''')

    header_y = y + 20
    div_y = y + 32
    res.append(f'<text x="{x + w/2}" y="{header_y}" class="t silm" font-size="10.0" font-weight="800" letter-spacing="2.4">COLORATION</text>')
    res.append(f'<line x1="{x + 14}" y1="{div_y}" x2="{x + w - 14}" y2="{div_y}" stroke="#26262C" stroke-width="1"/>')

    amp_cy = y + 84
    trans_cy = y + 164
    res.append(draw_segmented_selector(amp_param, x + w/2, amp_cy, "AMPLIFIER", amp_alias))
    res.append(draw_segmented_selector(trans_param, x + w/2, trans_cy, "TRANSFORMER", trans_alias))
    res.append(f'<text x="{x + w/2}" y="{y + 226}" class="t silm" font-size="9.5" font-weight="700" letter-spacing="1.8">HARMONIC DRIVE</text>')
    res.append(draw_vintage_push_button(push_param, x + w/2, y + 266, label="PUSH", state="off", width=44, height=44))

    res.append('</g>')
    return "\n".join(res)


def compose_console():
    """Generates the full 1920x1080 Masteraid SVG console document with MQN Tactile Suite assets."""
    t = load_theme()
    svg = []
    svg.append(f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {HEIGHT}" width="{WIDTH}" height="{HEIGHT}">
<defs>
  <!-- MAIN CHASSIS ANODIZED DARK GRADIENT -->
  <linearGradient id="panel" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#0E0E11"/>
    <stop offset="100%" stop-color="#070709"/>
  </linearGradient>

  <!-- BRUSHED GRAIN TEXTURE FILTER -->
  <filter id="brushed" x="0%" y="0%" width="100%" height="100%">
    <feTurbulence type="fractalNoise" baseFrequency="{t['brushed_freq_x']} {t['brushed_freq_y']}" numOctaves="3" seed="7" result="noise" />
    <feColorMatrix type="matrix" values="0 0 0 0 1   0 0 0 0 1   0 0 0 0 1   0 0 0 0 0.95 -0.30" result="streaks"/>
  </filter>

  <filter id="shadow-card" x="-5%" y="-5%" width="110%" height="110%">
    <feDropShadow dx="0" dy="8" stdDeviation="10" flood-color="#000000" flood-opacity="0.75" />
  </filter>

  <!-- MQN LATCH BUTTON GRADIENTS (MAGENTA LIT / METALLIC UNLIT) -->
  <linearGradient id="lens-amber-lit" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#FF4DA6"/>
    <stop offset="30%" stop-color="#FF007F"/>
    <stop offset="100%" stop-color="#99004D"/>
  </linearGradient>

  <linearGradient id="lens-amber-unlit" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#2E2E34"/>
    <stop offset="100%" stop-color="#1A1A1E"/>
  </linearGradient>

  <radialGradient id="jewel-magenta" cx="35%" cy="35%" r="65%">
    <stop offset="0%" stop-color="#FF99CC"/>
    <stop offset="35%" stop-color="#FF007F"/>
    <stop offset="80%" stop-color="#800040"/>
    <stop offset="100%" stop-color="#2A0015"/>
  </radialGradient>

  <linearGradient id="screw-grad" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#4A4A52"/>
    <stop offset="50%" stop-color="#202024"/>
    <stop offset="100%" stop-color="#383840"/>
  </linearGradient>

  {get_knob_defs()}
  {get_meter_defs()}
  {get_button_defs()}

  <style>
    @font-face {{
      font-family: 'DSEG14 Classic';
      src: url('data:font/woff2;base64,{FONT_B64}') format('woff2');
      font-weight: 700;
      font-style: normal;
    }}
    @font-face {{
      font-family: 'DSEG14 Classic Italic';
      src: url('data:font/woff2;base64,{FONT_B64}') format('woff2');
      font-weight: 700;
      font-style: normal;
    }}

    .t {{
      text-anchor: middle;
      dominant-baseline: central;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
    }}
    .mono {{
      font-family: "SF Mono", Menlo, Consolas, monospace;
    }}
    .silm {{
      fill: #8A8A92;
    }}
    .model-active {{
      fill: {ACCENT_MAGENTA};
      font-weight: 800;
      font-size: 10.5px;
      letter-spacing: 1.5px;
    }}
    .model-inactive {{
      fill: #8A8A92;
      font-weight: 700;
      font-size: 10.5px;
      letter-spacing: 1.5px;
      opacity: 0.55;
    }}

    /* MQN BUTTONS AND INDICATORS */
    .dot-amber {{
      fill: {ACCENT_MAGENTA};
      filter: url(#pink_glow);
    }}
    .dot-amber-p {{
      fill: #3A3A42;
      opacity: 0.6;
    }}
    .text-amber-active {{
      fill: #ECE9E1;
      font-weight: 800;
      font-size: 10px;
      letter-spacing: 1.5px;
    }}
    {get_button_css(is_dark=True)}

    /* CONSOLIDATED 14-SEGMENT HARDWARE DISPLAY (DSEG14 Classic Italic - Native Slant, No Faux Oblique) */
    .dseg14-ghost {{
      font-family: 'DSEG14 Classic', 'DSEG14 Classic Italic', monospace;
      font-style: normal;
      font-synthesis: none;
      font-size: 13.8px;
      font-weight: 700;
      fill: {ACCENT_MAGENTA};
      opacity: 0.16;
      text-anchor: middle;
      dominant-baseline: central;
    }}
    .dseg14-active {{
      font-family: 'DSEG14 Classic', 'DSEG14 Classic Italic', monospace;
      font-style: normal;
      font-synthesis: none;
      font-size: 13.8px;
      font-weight: 700;
      fill: #FF2491;
      text-anchor: middle;
      dominant-baseline: central;
      filter: url(#dseg_bloom);
    }}
    .dseg14-dot {{
      fill: #FF2491;
      filter: url(#dseg_bloom);
    }}
  </style>
</defs>''')

    # =========================================================================
    # 1. MAIN RACK CHASSIS & MOUNTING SCREWS (1640 x 940 Balanced Rack)
    # =========================================================================
    svg.append(f'<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#panel)"/>')
    svg.append(f'<rect width="{WIDTH}" height="{HEIGHT}" filter="url(#brushed)" opacity="0.08" pointer-events="none"/>')
    svg.append(f'<rect x="1" y="1" width="{WIDTH-2}" height="{HEIGHT-2}" fill="none" stroke="#26262C" stroke-width="1"/>')

    for sx in [17, WIDTH - 17]:
        for sy in [24, 320, 620, HEIGHT - 24]:
            svg.append(draw_screw(sx, sy))

    # =========================================================================
    # 2. SPLIT TOP STRIP: ANALOG TAPE (LEFT) & MASTERING CLIPPER (RIGHT)
    # =========================================================================
    top_x, top_y, top_w, top_h = 34, 14, 860, 128
    svg.append(draw_card("tape_unit", top_x, top_y, top_w, top_h, has_header_divider=True))
    svg.append(f'<text x="{top_x + 132}" y="{top_y + 20}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="3.2">ANALOG TAPE MASTERING</text>')
    svg.append(draw_model_select("tape_speed", top_x + 410, top_y + 20, "15 IPS", "30 IPS", "B", width=136))
    svg.append(draw_model_select("tape_formula", top_x + 556, top_y + 20, "456", "GP9", "A", width=116))
    svg.append(draw_vintage_bypass_button("tape_bypass", top_x + top_w - 54, top_y + 20, "IN", True))

    tape_cy = top_y + 86
    # 5 MQN Small Pots across Tape Strip Body (below header divider, 160px spacing)
    tape_freq_steps = ["30Hz", "40Hz", "50Hz", "60Hz", "70Hz", "80Hz", "90Hz", "105Hz", "120Hz", "135Hz", "150Hz"]
    flutter_steps = ["0%", "5%", "10%", "15%", "20%", "25%", "30%", "40%", "50%", "75%", "100%"]
    svg.append(draw_knob_small("tape_input", top_x + 110, tape_cy, "DRIVE", angle=27, readout_val="+2.0dB"))
    svg.append(draw_knob_small("tape_bias", top_x + 270, tape_cy, "BIAS", angle=-27, readout_val="-2.0dB"))
    svg.append(draw_knob_small("tape_headbump", top_x + 430, tape_cy, "HEAD BUMP", angle=54, readout_val="+4.0dB"))
    svg.append(draw_knob_small("tape_freq", top_x + 590, tape_cy, "BUMP FREQ", steps=tape_freq_steps, active_idx=5, readout_val="80Hz"))
    svg.append(draw_knob_small("tape_flutter", top_x + 750, tape_cy, "FLUTTER", steps=flutter_steps, active_idx=3, readout_val="15%"))

    # Right Top Card: MASTERING CLIPPER (x: 910, y: 14, w: 416, h: 128)
    clip_x, clip_y, clip_w, clip_h = 910, 14, 416, 128
    svg.append(draw_card("clipper_section", clip_x, clip_y, clip_w, clip_h, has_header_divider=True))
    svg.append(f'<text x="{clip_x + 112}" y="{clip_y + 20}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="3.0">MASTERING CLIPPER</text>')
    svg.append(draw_vintage_bypass_button("clipper_bypass", clip_x + clip_w - 54, clip_y + 20, "IN", True))

    ceil_steps = ["-3.0dB", "-2.0dB", "-1.5dB", "-1.0dB", "-0.7dB", "-0.5dB", "-0.3dB", "-0.2dB", "-0.1dB", "-0.05", "0.0dB"]
    svg.append(draw_knob_small("clipper_drive", clip_x + 76, tape_cy, "DRIVE", angle=27, readout_val="+2.0dB"))
    svg.append(draw_knob_small("clipper_ceiling", clip_x + 190, tape_cy, "CEILING", steps=ceil_steps, active_idx=6, readout_val="-0.3dB"))
    svg.append(draw_model_select("clipper_shape", clip_x + 322, top_y + 66, "SOFT", "HARD", "A", width=124))
    svg.append(draw_model_select("clipper_os", clip_x + 322, top_y + 102, "2X", "4X", "B", width=124))

    # =========================================================================
    # 3. THREE-COLUMN BALANCED MAIN DECK (y: 156, h: 782)
    # =========================================================================
    main_y = 156
    main_h = 782

    # -------------------------------------------------------------------------
    # 3.1 LEFT WING: INPUT & BASS MONOIZER (x: 34, w: 200)
    # -------------------------------------------------------------------------
    lw_x, lw_w = 34, 200
    lw_cx = lw_x + lw_w / 2
    svg.append(draw_card("input_mono", lw_x, main_y, lw_w, main_h, has_header_divider=True))
    svg.append(f'<text x="{lw_cx}" y="{main_y + 20}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="3.2">INPUT STAGE</text>')

    # MQN Single Big Knob + Top 6-Cell Readout on Compact 136x172 Card Well (Static label: TRIM)
    svg.append(draw_knob_large("input_trim", lw_cx, main_y + 162, "TRIM", angle=0, readout_val="+0.0DB", readout_on_top=True, with_card=True))

    svg.append(draw_model_select("input_polarity", lw_cx, main_y + 256, "NORM", "INV", "A", width=144))
    svg.append(draw_input_ladder("input_meter_ladder", lw_cx - 42.0, main_y + 286, w=84.0, h=124.0, with_top_readout=True, readout_label="IN LVL", readout_val="-0.8DB"))

    # Divider for Bass Mono section
    svg.append(f'<line x1="{lw_x+16}" y1="{main_y+468}" x2="{lw_x+lw_w-16}" y2="{main_y+468}" stroke="#000000" stroke-opacity="0.7"/>')
    svg.append(f'<line x1="{lw_x+16}" y1="{main_y+469}" x2="{lw_x+lw_w-16}" y2="{main_y+469}" stroke="#383840" stroke-opacity="0.35"/>')
    svg.append(f'<text x="{lw_cx}" y="{main_y+490}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="3.0">BASS MONO</text>')

    # MQN Small Pots on 80x100 Beveled Cards (11-tick connected dial)
    mono_freq_steps = ["20Hz", "30Hz", "40Hz", "50Hz", "60Hz", "80Hz", "100Hz", "120Hz", "150Hz", "180Hz", "220Hz"]
    pct_steps = ["0%", "10%", "20%", "30%", "40%", "50%", "60%", "70%", "80%", "90%", "100%"]
    svg.append(draw_knob_small("mono_freq", lw_cx, main_y + 562, "FREQ", steps=mono_freq_steps, active_idx=4, readout_val="60Hz", with_card=True))
    svg.append(draw_model_select("mono_slope", lw_cx, main_y + 636, "12 dB", "24 dB", "B", width=144))
    svg.append(draw_knob_small("mono_amount", lw_cx, main_y + 716, "AMOUNT", steps=pct_steps, active_idx=7, readout_val="70%", with_card=True))

    # -------------------------------------------------------------------------
    # 3.2 COMPACTED CENTER CONSOLE (x: 250, w: 860)
    # -------------------------------------------------------------------------
    cw_x, cw_w = 250, 860

    # --- 3.2.1 UPPER RACK: MASTERING PARAMETRIC EQ (y: 132, h: 384) ---
    eq_h = 384
    svg.append(draw_card("master_eq", cw_x, main_y, cw_w, eq_h, has_header_divider=True))
    svg.append(f'<text x="{cw_x + 115}" y="{main_y + 20}" class="t silm" font-size="11.5" font-weight="800" letter-spacing="3.5">PARAMETRIC EQ</text>')
    svg.append(draw_model_select("eq_model", cw_x + cw_w / 2, main_y + 20, "PULTEC", "MASSIVE", "B", width=220))
    svg.append(draw_vintage_bypass_button("eq_bypass", cw_x + cw_w - 54, main_y + 20, "IN", True))

    eq_out_w = 184
    eq_out_x = cw_x + cw_w - eq_out_w - 20
    band_zone_w = eq_out_x - cw_x - 20
    band_spacing = band_zone_w / 4
    band_centers = [cw_x + 14 + band_spacing * (i + 0.5) for i in range(4)]

    bands = [
        ("eq_b1", "LOW",      "LOW",    band_centers[0], 2, 5, True),
        ("eq_b2", "LOW MID",  "LO MID", band_centers[1], 2, 2, True),
        ("eq_b3", "HIGH MID", "HI MID", band_centers[2], 4, 5, True),
        ("eq_b4", "HIGH",     "HIGH",   band_centers[3], 3, 4, True),
    ]

    q_steps = ["0.4", "0.5", "0.7", "0.9", "1.2", "1.5", "2.0", "2.5", "3.0", "4.0", "5.0"]

    for band_id, band_name, ro_label, bx, active_f_idx, active_g_idx, is_on in bands:
        card_top_y = main_y + 56
        cy = card_top_y + 108

        svg.append(f'<!-- ================= {band_name} ================= -->')
        svg.append(f'<g id="band_{band_id}">')

        # Complete MQN Dual Concentric Knob on Compact 136x172 Card Well
        svg.append(draw_knob_concentric(
            band_id,
            bx,
            cy,
            band_label=ro_label,
            active_f_idx=active_f_idx,
            active_g_idx=active_g_idx,
            is_on=is_on,
            with_card=True,
            readout_on_top=True,
        ))

        # Bandwidth Q: MQN Small Pot on 80x100 Beveled Card Well beneath Concentric Card (18px air gap)
        svg.append(draw_knob_small(f"{band_id}_q", bx, main_y + 300, "Q", steps=q_steps, active_idx=5, readout_val="1.5", with_card=True))

        svg.append('</g>')

    # EQ Dedicated Coloration Cassette (310px tall, matching Concentric + Q height)
    svg.append(draw_coloration_cassette("eq_output_stage", eq_out_x, main_y + 56, eq_out_w, 310, "eq_amp", "TRIODE", "eq_transformer", "NICKEL", "eq_push", is_tall=True))

    # --- 3.2.2 LOWER RACK: MASTERING DYNAMICS (y: 530, h: 384) ---
    comp_y = main_y + eq_h + 14
    comp_h = 384
    svg.append(draw_card("comp_rack", cw_x, comp_y, cw_w, comp_h, has_header_divider=True))
    svg.append(f'<text x="{cw_x + 115}" y="{comp_y + 20}" class="t silm" font-size="11.5" font-weight="800" letter-spacing="3.5">MASTERING DYNAMICS</text>')
    svg.append(draw_model_select("comp_model", cw_x + cw_w / 2, comp_y + 20, "FAIRCHILD", "VARI-MU", "A", width=220))
    svg.append(draw_vintage_bypass_button("comp_bypass", cw_x + cw_w - 54, comp_y + 20, "IN", True))

    # Top Row Left: Stereo Smoked-Glass Analog VU Meters (GR L / GR R) at native 1.0 scale (200x132 each, 20px gap)
    svg.append(draw_vu_meter("vu_left", cw_x + 22, comp_y + 74, "GR L", needle_deg=-11.0, scale=1.0))
    svg.append(draw_vu_meter("vu_right", cw_x + 242, comp_y + 74, "GR R", needle_deg=-11.0, scale=1.0))

    # Top Row Right-of-VUs: Hero Compressor Threshold (MQN Single Big Knob on Compact 136x172 Card Well)
    svg.append(draw_knob_large("comp_thresh", cw_x + 548, comp_y + 164, "THRESH", angle=-27, readout_val="-2.0DB", readout_on_top=True, with_card=True))

    # Bottom Row: 6 MQN Small Pots on 80x100 Cards (SC HPF, MIX, MAKEUP, RATIO, ATTACK, RELEASE) with 22px air between cards
    sc_hpf_steps = ["OFF", "20Hz", "30Hz", "45Hz", "60Hz", "80Hz", "100Hz", "120Hz", "150Hz", "200Hz", "250Hz"]
    ratio_steps = ["1.2:1", "1.5:1", "1.8:1", "2.0:1", "2.5:1", "3.0:1", "4.0:1", "6.0:1", "8.0:1", "10:1", "20:1"]
    attack_steps = ["0.1ms", "0.2ms", "0.5ms", "1.0ms", "2.0ms", "3.0ms", "5.0ms", "10ms", "15ms", "20ms", "30ms"]
    release_steps = ["50ms", "80ms", "120ms", "200ms", "350ms", "500ms", "750ms", "1.0s", "1.5s", "2.0s", "AUTO"]

    comp_pots = [
        ("comp_sc_hpf",  "SC HPF",  sc_hpf_steps,  5,  None, "80Hz"),
        ("comp_mix",     "MIX",     pct_steps,     10, None, "100%"),
        ("comp_makeup",  "MAKEUP",  None,          5,  27,   "+2.0dB"),
        ("comp_ratio",   "RATIO",   ratio_steps,   5,  None, "3.0:1"),
        ("comp_attack",  "ATTACK",  attack_steps,  3,  None, "1.0ms"),
        ("comp_release", "RELEASE", release_steps, 7,  None, "1.0s"),
    ]
    for idx_p, (p_id, p_lbl, p_steps, p_aidx, p_ang, p_ro) in enumerate(comp_pots):
        px = cw_x + 62 + idx_p * 103
        if p_steps is not None:
            svg.append(draw_knob_small(p_id, px, comp_y + 300, p_lbl, steps=p_steps, active_idx=p_aidx, readout_val=p_ro, with_card=True))
        else:
            svg.append(draw_knob_small(p_id, px, comp_y + 300, p_lbl, angle=p_ang, readout_val=p_ro, with_card=True))

    # Compressor Coloration Cassette (Aligned vertically with EQ Coloration Cassette, identical 310px height)
    svg.append(draw_coloration_cassette("comp_output_stage", eq_out_x, comp_y + 56, eq_out_w, 310, "comp_amp", "TRIODE", "comp_transformer", "STEEL", "comp_push", is_tall=False))

    # -------------------------------------------------------------------------
    # 3.3 RIGHT WING: LIMITER & MASTER OUTPUT (x: 1126, w: 200)
    # -------------------------------------------------------------------------
    rw_x, rw_w = 1126, 200
    rw_cx = rw_x + rw_w / 2
    svg.append(draw_card("limiter_master", rw_x, main_y, rw_w, main_h, has_header_divider=True))
    svg.append(f'<text x="{rw_cx}" y="{main_y + 20}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="3.0">BRICKWALL LIMITER</text>')

    # Two Side-by-Side MQN Small Pots on 80x100 Cards (with 12px air between them)
    svg.append(draw_knob_small("limiter_gain", rw_cx - 45, main_y + 106, "GAIN", angle=54, readout_val="+4.0dB", with_card=True))
    svg.append(draw_knob_small("limiter_ceiling", rw_cx + 45, main_y + 106, "CEILING", steps=ceil_steps, active_idx=8, readout_val="-0.1dB", with_card=True))

    svg.append(draw_model_select("limiter_release", rw_cx, main_y + 184, "AUTO", "MAN", "A", width=168))
    svg.append(draw_model_select("limiter_truepeak", rw_cx, main_y + 222, "TRUE PK", "OFF", "A", width=168))

    svg.append(f'<line x1="{rw_x+16}" y1="{main_y+254}" x2="{rw_x+rw_w-16}" y2="{main_y+254}" stroke="#000000" stroke-opacity="0.7"/>')
    svg.append(f'<line x1="{rw_x+16}" y1="{main_y+255}" x2="{rw_x+rw_w-16}" y2="{main_y+255}" stroke="#383840" stroke-opacity="0.35"/>')
    svg.append(f'<text x="{rw_cx}" y="{main_y+276}" class="t silm" font-size="11.0" font-weight="800" letter-spacing="2.8">MASTER OUTPUT</text>')

    # Digital LUFS & True Peak 14-Segment Display
    svg.append(draw_lufs_display("lufs_display", rw_cx - 84, main_y + 296, w=168, h=70))

    # Master Output Gain: MQN Single Big Knob on Compact 136x172 Card Well (Static label: OUTPUT)
    svg.append(draw_knob_large("master_output", rw_cx, main_y + 498, "OUTPUT", angle=0, readout_val="+0.0DB", readout_on_top=True, with_card=True))

    # Preset Bar
    svg.append(f'''<g id="preset_bar" transform="translate({rw_cx - 84}, {main_y + 586})">
      <rect x="0" y="0" width="168" height="32" rx="6" fill="#0B0B0D" stroke="#26262C" stroke-width="1.2"/>
      <text x="84" y="16" class="t" font-size="9.5" font-weight="800" letter-spacing="1.4" fill="{ACCENT_MAGENTA}">01 CLEAN MASTER</text>
    </g>''')

    # Master Power Switch with Magenta Pilot Jewel
    svg.append(f'''<g id="master_power_section" transform="translate({rw_cx}, {main_y + 668})">
      <rect x="-70" y="-23" width="140" height="46" rx="8" fill="#0B0B0D" stroke="#26262C" stroke-width="1.2"/>
      <circle cx="-36" cy="0" r="10.0" fill="url(#jewel-magenta)" stroke="#4A0025" stroke-width="1.2" filter="url(#pink_glow)"/>
      <circle cx="-36" cy="0" r="12.0" fill="none" stroke="#383840" stroke-width="0.8"/>
      <rect x="4" y="-11" width="52" height="22" rx="4" fill="url(#mqn_rotor_main_grad)" stroke="#383840" stroke-width="0.8"/>
      <circle cx="20" cy="0" r="7" fill="url(#screw-grad)" stroke="#111" stroke-width="1"/>
      <line x1="20" y1="0" x2="38" y2="-6" stroke="#ECE9E1" stroke-width="3.2" stroke-linecap="round"/>
      <text x="0" y="38" class="t silm" font-size="9.5" font-weight="800" letter-spacing="2.0">MASTER POWER</text>
    </g>''')

    svg.append('</svg>')
    return "\n".join(svg)
