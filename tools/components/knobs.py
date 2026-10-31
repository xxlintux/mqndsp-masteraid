"""
Masteraid MQN Tactile Suite Knob & Segmented Readout Components
Implements the official MQN Tactile Design System:
- Primary Accent: Pure Neon Magenta (#FF007F)
- Consolidated 14-Segment Alphanumeric Starburst Hardware Display (Recessed bezel well, smoked magenta-tinted glass lens, subtle glowing inner rim, top optical glass reflection, visible ~ 14-segment ghost grid, soft hardware LED/VFD bloom, and decimal-attached cell tokenization).
- Single Big Knob (R = 46.0px, 92px diameter): Flat monolithic machined rotor, 4x23px magenta notch, 11-tick connected dial (-135..+135 deg in 27 deg steps, R=46.5..54.0), and 6-cell 14-segment starburst readout.
- Dual Concentric Knob (R_outer = 46.0px knurled collar with 2px protruding magenta pointer; R_inner = 40.0px flat monolithic inner rotor with 4x20px magenta notch), 9 upper gain ticks (-90..+90 deg in 22.5 deg steps, R=46.5..54.0) + 6 lower frequency dots (R=54.0, [150, 126, 102, 78, 54, 30] deg polar), zero static values on pots, and 6-cell 14-segment readout.
- Small Pot (R = 23.0px, exact half radius): Concentric circular base skirt (R = 23.0px) underneath a bold 6-flute scalloped head (dipping to R = 18.0px) with step shadow, 2.6x11.5px magenta notch on the front crest, connected 11-tick dial (-135..+135 deg in 27 deg steps, R=24.0..30.5) with pure magenta active/middle tick, and label/readout.
"""

import math
from components.displays import (
    ACCENT_MAGENTA,
    draw_consolidated_segmented_display,
    get_display_defs,
    tokenize_dseg_cells,
)

TICK_ANGLES_11 = [-135, -108, -81, -54, -27, 0, 27, 54, 81, 108, 135]
CONCENTRIC_GAIN_DEGS = [-90.0, -67.5, -45.0, -22.5, 0.0, 22.5, 45.0, 67.5, 90.0]
CONCENTRIC_FREQ_POLAR_DEGS = [150.0, 126.0, 102.0, 78.0, 54.0, 30.0]


def get_knob_defs():
    """Returns SVG <defs> for the MQN Tactile Suite knobs, plates, and consolidated segmented readouts."""
    return f'''
  <!-- MQN UNIVERSAL PLATE WELL METALLIC GRADIENT -->
  <linearGradient id="mqn_plate_well_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#141416"/>
    <stop offset="100%" stop-color="#0B0B0D"/>
  </linearGradient>

  <!-- MQN RECESSED SUB-PANEL GRADIENT -->
  <linearGradient id="mqn_subpanel_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#111114"/>
    <stop offset="100%" stop-color="#09090B"/>
  </linearGradient>

  <!-- MQN MONOLITHIC FLAT ROTOR BODY GRADIENT -->
  <linearGradient id="mqn_rotor_main_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#3A3A40"/>
    <stop offset="100%" stop-color="#202024"/>
  </linearGradient>

  <!-- MQN CONCENTRIC OUTER KNURLED COLLAR GRADIENT -->
  <linearGradient id="mqn_outer_collar_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#424248"/>
    <stop offset="50%" stop-color="#28282D"/>
    <stop offset="100%" stop-color="#161619"/>
  </linearGradient>

  <!-- MQN SMALL POT CIRCULAR BASE SKIRT GRADIENT -->
  <linearGradient id="mqn_small_pot_skirt_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#2C2C32"/>
    <stop offset="100%" stop-color="#161619"/>
  </linearGradient>

  <!-- MQN STATIC WORLD-SPACE TOP-DOWN LIGHTING OVERLAY (NEVER ROTATES WITH KNOB) -->
  <linearGradient id="mqn_static_top_light" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.16"/>
    <stop offset="45%" stop-color="#FFFFFF" stop-opacity="0.0"/>
    <stop offset="100%" stop-color="#000000" stop-opacity="0.30"/>
  </linearGradient>

  <!-- MQN STATIC SPECULAR RIM HIGHLIGHT GRADIENT (TOP LIT, BOTTOM SHADED) -->
  <linearGradient id="mqn_rim_light_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.28"/>
    <stop offset="50%" stop-color="#FFFFFF" stop-opacity="0.06"/>
    <stop offset="100%" stop-color="#000000" stop-opacity="0.35"/>
  </linearGradient>

  <!-- MQN BIG ROTOR DROP SHADOW -->
  <filter id="mqn_rotor_shadow" x="-30%" y="-25%" width="160%" height="170%">
    <feDropShadow dx="0" dy="6" stdDeviation="6" flood-color="#000000" flood-opacity="0.65"/>
  </filter>

  <!-- MQN SMALL POT DROP SHADOW -->
  <filter id="mqn_small_pot_shadow" x="-30%" y="-25%" width="160%" height="170%">
    <feDropShadow dx="0" dy="2.2" stdDeviation="2.2" flood-color="#000000" flood-opacity="0.6"/>
  </filter>

  <!-- MQN SMALL POT SCALLOP STEP SHADOW ONTO CIRCULAR SKIRT -->
  <filter id="mqn_scallop_step_shadow" x="-20%" y="-20%" width="140%" height="140%">
    <feDropShadow dx="0" dy="1.0" stdDeviation="1.0" flood-color="#000000" flood-opacity="0.55"/>
  </filter>

  <!-- MQN PURE MAGENTA LED GLOW -->
  <filter id="pink_glow" x="-25%" y="-25%" width="150%" height="150%">
    <feGaussianBlur stdDeviation="0.8" result="blur"/>
    <feMerge>
      <feMergeNode in="blur"/>
      <feMergeNode in="SourceGraphic"/>
    </feMerge>
  </filter>
  {get_display_defs(is_dark=True)}
'''


def generate_bold_scallop_path(cx, cy, r_crest=23.0, crest_half_angle=8.0, r_scoop_mult=0.75):
    """Generates continuous 6-flute scalloped perimeter with 5px deep concave scoops."""
    path_cmds = []
    for i in range(6):
        c_deg = i * 60.0
        next_c_deg = (i + 1) * 60.0

        a_c_start = math.radians(c_deg - crest_half_angle - 90.0)
        a_c_end = math.radians(c_deg + crest_half_angle - 90.0)
        a_nc_start = math.radians(next_c_deg - crest_half_angle - 90.0)

        p_c_end_x = cx + r_crest * math.cos(a_c_end)
        p_c_end_y = cy + r_crest * math.sin(a_c_end)
        p_nc_start_x = cx + r_crest * math.cos(a_nc_start)
        p_nc_start_y = cy + r_crest * math.sin(a_nc_start)

        if i == 0:
            start_x = cx + r_crest * math.cos(a_c_start)
            start_y = cy + r_crest * math.sin(a_c_start)
            path_cmds.append(f"M {start_x:.2f} {start_y:.2f}")

        path_cmds.append(f"A {r_crest:.2f} {r_crest:.2f} 0 0 1 {p_c_end_x:.2f} {p_c_end_y:.2f}")

        chord = math.hypot(p_nc_start_x - p_c_end_x, p_nc_start_y - p_c_end_y)
        r_scoop = chord * r_scoop_mult
        path_cmds.append(f"A {r_scoop:.2f} {r_scoop:.2f} 0 0 0 {p_nc_start_x:.2f} {p_nc_start_y:.2f}")

    path_cmds.append("Z")
    return " ".join(path_cmds)


def draw_plate_well_160x200(x, y, w=136, h=172):
    """Draws the compact 136x172 MQN beveled metallic card well for Big and Concentric Knobs."""
    return f'''<g class="mqn_plate_big" transform="translate({x:.1f}, {y:.1f})">
      <rect x="0" y="0" width="{w}" height="{h}" rx="16" fill="url(#mqn_plate_well_grad)" stroke="#26262C" stroke-width="1"/>
      <rect x="1" y="1" width="{w - 2}" height="{h - 2}" rx="15" fill="none" stroke="#383840" stroke-width="0.7" stroke-opacity="0.45"/>
    </g>'''


def draw_plate_well_80x100(x, y):
    """Draws the 80x100 MQN compact beveled metallic card well matching knob_small_pot_card_label_top.svg."""
    return f'''<g class="mqn_plate_80x100" transform="translate({x:.1f}, {y:.1f})">
      <rect x="0" y="0" width="80" height="100" rx="12" fill="url(#mqn_plate_well_grad)" stroke="#26262C" stroke-width="1"/>
      <rect x="1" y="1" width="78" height="98" rx="11" fill="none" stroke="#383840" stroke-width="0.7" stroke-opacity="0.45"/>
    </g>'''


def draw_6cell_readout(id_name, cx, cy, label="TREBLE", momentary_val="+0.0DB"):
    """
    Draws the consolidated 84x32px 6-cell 14-segment starburst readout label centered at (cx, cy).
    Delegates directly to the upstream `components.displays.draw_consolidated_segmented_display`.
    """
    return draw_consolidated_segmented_display(
        id_name=id_name,
        cx=cx,
        cy=cy,
        width=84.0,
        height=32.0,
        num_cells=6,
        text_str=label,
        momentary_val=momentary_val,
        char_id_prefix="seg_char",
        rx_outer=8.0,
    )


def draw_knob_concentric(band_id, cx, cy, band_label="LOW", active_f_idx=3, active_g_idx=4, is_on=True, with_card=True, readout_on_top=True):
    """
    MQN Dual Concentric Knob + Consolidated 6-Cell Segmented Readout:
    - 160x200 Beveled Card Plate Well (when with_card=True)
    - Consolidated 6-Cell 14-Segment Readout displaying static `band_label` (momentarily shows gain/freq on change)
    - Connected Upper Gain Ticks (R=46.5..54.0px, 9 ticks at [-90, -67.5, -45, -22.5, 0, 22.5, 45, 67.5, 90])
    - 6 Lower Frequency Step Dots at R=54.0px (polar [150, 126, 102, 78, 54, 30] deg)
    - Zero static values shown around the pot
    - Outer Knurled Collar (R=46.0px) with 2px protruding magenta pointer
    - Inner Main Rotor (R=40.0px) flat monolithic cap with 4x20px rounded magenta indicator notch
    """
    res = [f'<!-- MQN DUAL CONCENTRIC KNOB ASSEMBLY: {band_id} -->']

    if with_card:
        card_x = cx - 68.0
        card_y = (cy - 108.0) if readout_on_top else (cy - 66.0)
        ro_cy = (cy - 78.0) if readout_on_top else (cy + 76.0)
        res.append(draw_plate_well_160x200(card_x, card_y, w=136, h=172))
    else:
        ro_cy = (cy - 76.0) if readout_on_top else (cy + 74.0)

    # 1. Consolidated 6-Cell 14-Segment Readout Label
    res.append(draw_6cell_readout(band_id, cx, ro_cy, label=band_label, momentary_val="+0.0DB"))

    # 2. Concentric Ticks & Dots (NO static text around pot)
    r_outer_arc = 54.0
    res.append(f'<g id="dial_ticks_concentric_{band_id}">')
    res.append(
        f'<path id="gain_outerline_halfcircle_{band_id}" '
        f'd="M {cx - r_outer_arc:.1f} {cy:.1f} A {r_outer_arc:.1f} {r_outer_arc:.1f} 0 0 1 {cx + r_outer_arc:.1f} {cy:.1f}" '
        f'fill="none" stroke="#ECE9E1" stroke-width="0.9" stroke-opacity="0.35"/>'
    )

    for idx, deg in enumerate(CONCENTRIC_GAIN_DEGS):
        rad = math.radians(deg - 90.0)
        x1 = cx + 46.5 * math.cos(rad)
        y1 = cy + 46.5 * math.sin(rad)
        x2 = cx + 54.0 * math.cos(rad)
        y2 = cy + 54.0 * math.sin(rad)
        is_sel = (idx == active_g_idx) and is_on
        res.append(
            f'<g id="gslot_{band_id}_{idx}" data-deg="{deg}" style="cursor: pointer;">'
            f'<line id="gain_tick_{band_id}_{idx}" data-deg="{deg}" data-selected="{"true" if is_sel else "false"}" '
            f'x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
            f'stroke="{ACCENT_MAGENTA if is_sel else "#ECE9E1"}" stroke-width="1.4" stroke-linecap="round" '
            f'stroke-opacity="{"1.0" if is_sel else "0.45"}"/>'
            f'</g>'
        )

    for idx, polar_deg in enumerate(CONCENTRIC_FREQ_POLAR_DEGS):
        rad = math.radians(polar_deg)
        fx = cx + 54.0 * math.cos(rad)
        fy = cy + 54.0 * math.sin(rad)
        is_sel = (idx == active_f_idx) and is_on
        res.append(
            f'<g id="fslot_{band_id}_{idx}" data-deg="{polar_deg}" style="cursor: pointer;">'
            f'<circle id="freq_dot_{band_id}_{idx}" cx="{fx:.1f}" cy="{fy:.1f}" '
            f'r="{"2.2" if is_sel else "1.4"}" fill="{ACCENT_MAGENTA if is_sel else "#8A8A90"}" '
            f'opacity="{"1.0" if is_sel else "0.45"}"/>'
            f'</g>'
        )
    res.append('</g>')

    # 3. Outer Knurled Collar (R = 46.0px): Static Lighting/Body + Rotating Knurl/Pointer
    f_angle = CONCENTRIC_FREQ_POLAR_DEGS[active_f_idx]
    g_angle = CONCENTRIC_GAIN_DEGS[active_g_idx]
    ptr_color = ACCENT_MAGENTA if is_on else "#55555E"

    res.append(f'''<g id="static_collar_{band_id}" pointer-events="none">
      <circle cx="{cx:.1f}" cy="{cy + 1.5:.1f}" r="46.0" fill="#000000" opacity="0.5" filter="url(#mqn_rotor_shadow)"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="46.0" fill="url(#mqn_outer_collar_grad)" stroke="#17171A" stroke-width="1.0"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="45.3" fill="none" stroke="url(#mqn_rim_light_grad)" stroke-width="0.7"/>
    </g>''')
    res.append(f'''<g id="knob_{band_id}_freq" data-param="{band_id}_freq" data-type="stepped-freq" data-band="{band_id}" cx="{cx}" cy="{cy}" transform="translate({cx}, {cy}) rotate({f_angle})">
      <circle cx="0" cy="0" r="44.5" fill="none" stroke="#000000" stroke-width="2.5" stroke-dasharray="1.8 2.5" stroke-opacity="0.6"/>
      <!-- Protruding 2px Frequency Pointer (R=39.5 to R=48.5 along +X axis to match polar rotate(f_angle)) -->
      <line x1="39.5" y1="0" x2="48.5" y2="0" stroke="#000000" stroke-width="4.0" stroke-linecap="round" opacity="0.7"/>
      <line x1="40.0" y1="0" x2="48.0" y2="0" stroke="{ptr_color}" stroke-width="2.6" stroke-linecap="round"/>
      <circle cx="0" cy="0" r="45" fill="none" stroke="transparent" stroke-width="12" style="cursor: pointer;"/>
    </g>''')

    # 4. Inner Main Rotor (R = 40.0px): Static Lighting/Body + Rotating Gain Indicator Notch
    notch_color = ACCENT_MAGENTA if is_on else "#6E6E77"
    res.append(f'''<g id="static_inner_{band_id}" pointer-events="none">
      <circle cx="{cx:.1f}" cy="{cy + 1.0:.1f}" r="40.5" fill="#000000" opacity="0.45"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="40.0" fill="url(#mqn_rotor_main_grad)" stroke="#18181C" stroke-width="0.9"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="39.2" fill="none" stroke="url(#mqn_rim_light_grad)" stroke-width="0.7"/>
    </g>''')
    res.append(f'''<g id="knob_{band_id}_gain" data-param="{band_id}_gain" data-type="stepped-gain" data-band="{band_id}" cx="{cx}" cy="{cy}" transform="translate({cx}, {cy}) rotate({g_angle})">
      <!-- Rounded Magenta Gain Indicator Notch -->
      <rect x="-2.5" y="-34.5" width="5.0" height="21.0" rx="2.5" fill="#000000" opacity="0.5"/>
      <rect x="-2.0" y="-34.0" width="4.0" height="20.0" rx="2.0" fill="{notch_color}"/>
      <circle cx="0" cy="0" r="39" fill="transparent" style="cursor: ns-resize;"/>
    </g>''')

    return "\n".join(res)


def draw_knob_large(id_name, cx, cy, label, angle=0, ktype="continuous", steps=None, active_idx=5, readout_val="+0.0DB", readout_on_top=True, with_card=True):
    """
    MQN Single Big Knob (R = 46.0px, 92.0px diameter) with 11-tick connected dial (-135..+135 deg in 27 deg steps)
    and consolidated 6-cell 14-segment readout showing static `label` (momentarily shows value changes).
    """
    res = [f'<!-- MQN SINGLE BIG KNOB: {id_name} -->']

    if with_card:
        card_x = cx - 68.0
        card_y = (cy - 108.0) if readout_on_top else (cy - 66.0)
        ro_cy = (cy - 78.0) if readout_on_top else (cy + 76.0)
        res.append(draw_plate_well_160x200(card_x, card_y, w=136, h=172))
    else:
        ro_cy = (cy - 76.0) if readout_on_top else (cy + 74.0)

    # 1. Consolidated 6-Cell 14-Segment Readout Box (displays static `label` by default)
    res.append(draw_6cell_readout(id_name, cx, ro_cy, label=label, momentary_val=readout_val))

    # 2. Connected 11-Tick Dial (R_in = 46.5 to R_arc = 54.0, -135 deg to +135 deg in 27 deg steps)
    R_in = 46.5
    R_arc = 54.0
    best_idx = min(range(11), key=lambda i: abs(TICK_ANGLES_11[i] - angle))
    angle = TICK_ANGLES_11[best_idx]

    rad_s = math.radians(-135 - 90)
    rad_e = math.radians(135 - 90)
    xs, ys = cx + R_arc * math.cos(rad_s), cy + R_arc * math.sin(rad_s)
    xe, ye = cx + R_arc * math.cos(rad_e), cy + R_arc * math.sin(rad_e)

    res.append(f'<g id="scale_{id_name}">')
    res.append(
        f'<path d="M {xs:.1f} {ys:.1f} A {R_arc:.1f} {R_arc:.1f} 0 1 1 {xe:.1f} {ye:.1f}" '
        f'fill="none" stroke="#ECE9E1" stroke-width="0.9" stroke-opacity="0.35" stroke-linecap="round"/>'
    )
    for idx, deg in enumerate(TICK_ANGLES_11):
        rad = math.radians(deg - 90)
        x1 = cx + R_in * math.cos(rad)
        y1 = cy + R_in * math.sin(rad)
        x2 = cx + R_arc * math.cos(rad)
        y2 = cy + R_arc * math.sin(rad)
        is_sel = (idx == best_idx)
        stroke_col = ACCENT_MAGENTA if is_sel else "#ECE9E1"
        stroke_op = "1.0" if is_sel else "0.45"
        res.append(
            f'<line id="dial_tick_{id_name}_{idx}" data-deg="{deg}" data-selected="{"true" if is_sel else "false"}" '
            f'x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
            f'stroke="{stroke_col}" stroke-width="1.4" stroke-linecap="round" stroke-opacity="{stroke_op}" style="cursor: pointer;"/>'
        )
    res.append('</g>')

    # 3. Interactive Single Big Knob (R = 46.0px): Static Lighting/Body + Rotating Indicator Notch
    res.append(f'<g id="knob_{id_name}" data-param="{id_name}" data-type="big-knob" data-static="{label}" cx="{cx}" cy="{cy}">')
    res.append(f'''<g id="static_body_{id_name}" pointer-events="none">
      <circle cx="{cx:.1f}" cy="{cy + 1.5:.1f}" r="46.5" fill="#000000" opacity="0.5" filter="url(#mqn_rotor_shadow)"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="46.0" fill="url(#mqn_rotor_main_grad)" stroke="#18181C" stroke-width="1.0"/>
      <circle cx="{cx:.1f}" cy="{cy:.1f}" r="45.2" fill="none" stroke="url(#mqn_rim_light_grad)" stroke-width="0.75"/>
    </g>''')
    res.append(f'''<g id="rotor_{id_name}" transform="translate({cx},{cy}) rotate({angle})">
      <rect x="-2.5" y="-40.5" width="5.0" height="23.0" rx="2.5" fill="#000000" opacity="0.5"/>
      <rect x="-2.0" y="-40.0" width="4.0" height="22.0" rx="2.0" fill="{ACCENT_MAGENTA}"/>
      <circle cx="0" cy="0" r="46" fill="transparent" style="cursor: ns-resize;"/>
    </g>''')
    res.append('</g>')

    return "\n".join(res)


def draw_knob_small(id_name, cx, cy, label, angle=0, ktype="continuous", steps=None, active_idx=5, readout_val=None, label_on_top=True, with_card=False):
    """
    MQN Small Pot (R = 23.0px, exact half radius of Big Knob):
    - Concentric circular base skirt (R = 23.0px) underneath bold 6-flute scalloped head
    - Pointer notch (2.6x11.5px, #FF007F) on front crest between -30 and +30 deg flutes
    - Connected 11-tick dial (R_arc = 30.5px, R_in = 24.0px, strictly [-135..+135] deg in 27 deg steps)
    - Label on top & Readout on bottom (or vice-versa), with ZERO static values around the ticks
    - Optional 80x100 beveled card plate well
    """
    res = [f'<!-- MQN SMALL POT: {id_name} -->']

    if with_card:
        card_x = cx - 40.0
        card_y = cy - 54.0
        res.append(draw_plate_well_80x100(card_x, card_y))
        y_top = cy - 38.5
        y_bot = cy + 32.0
    else:
        y_top = cy - 38.0
        y_bot = cy + 31.5

    if steps and len(steps) == 11:
        best_idx = max(0, min(10, active_idx))
        angle = TICK_ANGLES_11[best_idx]
        if readout_val is None:
            readout_val = str(steps[best_idx])
    else:
        best_idx = min(range(11), key=lambda i: abs(TICK_ANGLES_11[i] - angle))
        angle = TICK_ANGLES_11[best_idx]
        if readout_val is None:
            if angle == 0:
                readout_val = "0.0dB"
            else:
                readout_val = f"{angle / 13.5:+.1f}dB"

    # 1. Connected 11-Tick Dial (R_arc = 30.5, R_in = 24.0, -135..+135 deg in 27 deg steps)
    R_arc = 30.5
    R_in = 24.0
    rad_s = math.radians(-135.0 - 90.0)
    rad_e = math.radians(135.0 - 90.0)
    xs, ys = cx + R_arc * math.cos(rad_s), cy + R_arc * math.sin(rad_s)
    xe, ye = cx + R_arc * math.cos(rad_e), cy + R_arc * math.sin(rad_e)

    res.append(f'<g id="scale_{id_name}">')
    res.append(
        f'<path d="M {xs:.2f} {ys:.2f} A {R_arc:.2f} {R_arc:.2f} 0 1 1 {xe:.2f} {ye:.2f}" '
        f'fill="none" stroke="#ECE9E1" stroke-width="0.9" stroke-opacity="0.35" stroke-linecap="round"/>'
    )

    for idx, deg in enumerate(TICK_ANGLES_11):
        is_sel = (idx == best_idx)
        is_major = (deg == 0 or abs(deg) == 135)
        rad = math.radians(deg - 90.0)
        x1 = cx + R_in * math.cos(rad)
        y1 = cy + R_in * math.sin(rad)
        x2 = cx + R_arc * math.cos(rad)
        y2 = cy + R_arc * math.sin(rad)
        t_stroke = ACCENT_MAGENTA if is_sel else "#ECE9E1"
        t_op = "1.0" if is_sel else ("0.75" if is_major else "0.45")
        step_lbl = steps[idx] if (steps and idx < len(steps)) else ""
        res.append(
            f'<line id="small_pot_tick_{id_name}_{idx}" data-deg="{deg}" data-step-label="{step_lbl}" '
            f'x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" '
            f'stroke="{t_stroke}" stroke-width="1.4" stroke-linecap="round" stroke-opacity="{t_op}" style="cursor: pointer;"/>'
        )
    res.append('</g>')

    # 2. Small Pot (Static Circular Base Skirt + World-Space Step Shadow + Rotating Scalloped Head + Static Top Lighting)
    steps_json = f'data-step-list="{",".join(steps)}"' if steps else ''
    res.append(f'<g id="knob_{id_name}" data-param="{id_name}" data-type="small-pot" data-idx="{best_idx}" {steps_json} cx="{cx}" cy="{cy}">')

    main_scallop_d = generate_bold_scallop_path(0.0, 0.0, r_crest=23.0)
    inner_scallop_d = generate_bold_scallop_path(0.0, 0.0, r_crest=22.2, crest_half_angle=7.5)

    res.append(f'''<g id="static_skirt_{id_name}">
      <circle cx="{cx:.2f}" cy="{cy + 1.2:.2f}" r="23.0" fill="#000000" opacity="0.5" filter="url(#mqn_small_pot_shadow)"/>
      <circle cx="{cx:.2f}" cy="{cy:.2f}" r="23.0" fill="url(#mqn_small_pot_skirt_grad)" stroke="#18181C" stroke-width="0.8"/>
      <circle cx="{cx:.2f}" cy="{cy:.2f}" r="22.5" fill="none" stroke="url(#mqn_rim_light_grad)" stroke-width="0.55"/>
    </g>''')

    res.append(f'''<g filter="url(#mqn_scallop_step_shadow)">
      <g id="rotor_{id_name}" transform="translate({cx:.2f},{cy:.2f}) rotate({angle:.1f})">
        <path d="{main_scallop_d}" fill="#2B2B31" stroke="#18181C" stroke-width="0.8" stroke-linejoin="round"/>
        <path d="{inner_scallop_d}" fill="none" stroke="#FFFFFF" stroke-width="0.5" stroke-opacity="0.12" stroke-linejoin="round"/>
        <rect x="-1.7" y="-22.4" width="3.4" height="12.3" rx="1.7" fill="#000000" opacity="0.45"/>
        <rect x="-1.3" y="-22.0" width="2.6" height="11.5" rx="1.3" fill="{ACCENT_MAGENTA}"/>
      </g>
    </g>''')

    res.append(f'<circle cx="{cx:.2f}" cy="{cy:.2f}" r="23.0" fill="url(#mqn_static_top_light)" pointer-events="none"/>')

    # 3. Label & Tabular Monospace Readout Text
    lbl_y = y_top if label_on_top else y_bot
    ro_y = y_bot if label_on_top else y_top

    res.append(
        f'<text x="{cx:.1f}" y="{lbl_y:.1f}" '
        f'font-family="-apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, monospace" '
        f'font-size="8.5" font-weight="700" letter-spacing="1.2" '
        f'text-anchor="middle" dominant-baseline="central" fill="#8A8A92">{label}</text>'
    )
    res.append(
        f'<text id="readout_val_{id_name}" x="{cx:.1f}" y="{ro_y:.1f}" '
        f'font-family="\'SF Mono\', Consolas, Menlo, monospace" font-variant-numeric="tabular-nums" '
        f'font-size="8.5" font-weight="600" letter-spacing="0.4" '
        f'text-anchor="middle" dominant-baseline="central" fill="{ACCENT_MAGENTA}">{readout_val}</text>'
    )

    res.append('</g>')
    return "\n".join(res)


def draw_knob_medium(id_name, cx, cy, label, angle=0, ktype="continuous", steps=None, active_idx=5, readout_val=None, with_card=True):
    """Maps medium knob calls to the MQN Small Pot on its 80x100 card plate well."""
    return draw_knob_small(id_name, cx, cy, label, angle=angle, ktype=ktype, steps=steps, active_idx=active_idx, readout_val=readout_val, label_on_top=True, with_card=with_card)


def draw_knob_micro(id_name, cx, cy, label, angle=0, ktype="continuous", steps=None, active_idx=5, readout_val=None):
    """Maps micro knob calls to the floating MQN Small Pot."""
    return draw_knob_small(id_name, cx, cy, label, angle=angle, ktype=ktype, steps=steps, active_idx=active_idx, readout_val=readout_val, label_on_top=True, with_card=False)
