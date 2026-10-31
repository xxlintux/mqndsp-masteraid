"""
Masteraid MQN Tactile Suite Meter and Display Components
- Authentic MQN Analog VU Meter (200x132 beveled recessed housing, 9 calibrated tick values: -20, -10, -7, -5, -3, 0, +1, +2, +3, #FF007F overload zone)
- Consolidated 14-Segment Alphanumeric Starburst Hardware Display (uses draw_consolidated_segmented_display from components.knobs)
- Digital LUFS / True Peak Loudness Display & Multi-segment LED Ladders
"""

import math
from components.displays import ACCENT_MAGENTA, draw_consolidated_segmented_display


def get_meter_defs():
    """Returns SVG <defs> for MQN VU meters and displays."""
    return '''
  <!-- MQN VU METER GRADIENTS & FILTERS -->
  <linearGradient id="mqn_vu_bezel_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#2E2E33"/>
    <stop offset="100%" stop-color="#1C1C20"/>
  </linearGradient>

  <linearGradient id="mqn_vu_cavity_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#16060E"/>
    <stop offset="100%" stop-color="#0D0308"/>
  </linearGradient>

  <linearGradient id="mqn_vu_front_glass_tint" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#FF007F" stop-opacity="0.04"/>
    <stop offset="50%" stop-color="#16060E" stop-opacity="0.05"/>
    <stop offset="100%" stop-color="#0D0308" stop-opacity="0.12"/>
  </linearGradient>

  <linearGradient id="mqn_vu_glass_glare" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#FFFFFF" stop-opacity="0.12"/>
    <stop offset="45%" stop-color="#FFFFFF" stop-opacity="0.03"/>
    <stop offset="100%" stop-color="#FFFFFF" stop-opacity="0.0"/>
  </linearGradient>

  <linearGradient id="mqn_vu_needle_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#DFC5D2"/>
    <stop offset="60%" stop-color="#B898A8"/>
    <stop offset="100%" stop-color="#5E4652"/>
  </linearGradient>

  <linearGradient id="mqn_vu_pivot_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="#261820"/>
    <stop offset="100%" stop-color="#0E070B"/>
  </linearGradient>

  <filter id="mqn_vu_bezel_shadow" x="-15%" y="-15%" width="130%" height="135%">
    <feDropShadow dx="0" dy="6" stdDeviation="6" flood-color="#000000" flood-opacity="0.6"/>
  </filter>

  <filter id="mqn_vu_needle_shadow" x="-50%" y="-20%" width="200%" height="140%">
    <feDropShadow dx="2" dy="2" stdDeviation="1.5" flood-color="#000000" flood-opacity="0.6"/>
  </filter>

  <filter id="mqn_vu_arch_backlight" x="-30%" y="-30%" width="160%" height="160%">
    <feGaussianBlur in="SourceGraphic" stdDeviation="3.0" result="soft_glow"/>
  </filter>

  <filter id="mqn_vu_subtle_bloom" x="-25%" y="-25%" width="150%" height="150%">
    <feGaussianBlur in="SourceGraphic" stdDeviation="1.1" result="blur"/>
    <feColorMatrix in="blur" type="matrix" values="
      1 0 0 0 0
      0 1 0 0 0
      0 0 1 0 0
      0 0 0 0.45 0" result="dim_blur"/>
    <feMerge>
      <feMergeNode in="dim_blur"/>
      <feMergeNode in="SourceGraphic"/>
    </feMerge>
  </filter>
'''


def draw_vu_meter(vu_id, vx, vy, vtitle="GR L", needle_deg=-11.0, scale=1.15):
    """
    MQN Analog VU Meter (Native 200x132, scaled by `scale`):
    - Beveled dark metallic housing & recessed dial face
    - Connected tick arc (R=86.0px) centered at pivot (100, 118)
    - Safe arc (-44 to +11 deg) and Pure Magenta (#FF007F) overload arc (+11 to +44 deg)
    - All 9 calibrated tick marks and values: -20, -10, -7, -5, -3, 0, +1, +2, +3
    - Dynamic metallic needle & pivot cap rotating around (100, 118)
    """
    pivot_x = 100.0
    pivot_y = 118.0
    R_arc = 86.0
    R_label = 96.5

    def pt(deg, r):
        rad = math.radians(deg - 90.0)
        return pivot_x + r * math.cos(rad), pivot_y + r * math.sin(rad)

    tick_specs = [
        (-44, "-20", True,  False),
        (-33, "-10", False, False),
        (-22, "-7",  False, False),
        (-11, "-5",  False, False),
        (  0, "-3",  True,  False),
        ( 11, "0",   True,  False),
        ( 22, "+1",  False, True),
        ( 33, "+2",  False, True),
        ( 44, "+3",  True,  True),
    ]

    x_s, y_s = pt(-44, R_arc)
    x_m, y_m = pt(11, R_arc)
    x_e, y_e = pt(44, R_arc)

    res = [f'<!-- MQN ANALOG VU METER: {vu_id} -->']
    res.append(f'<g id="{vu_id}" transform="translate({vx:.1f},{vy:.1f}) scale({scale:.3f})">')

    # 1. Outer Machined Frame (200x132, rx=16) & Deep Recessed Bezel Trench (194x126 at 3,3, rx=13.5)
    res.append(
        '<rect x="0" y="0" width="200" height="132" rx="16" '
        'fill="#16161B" stroke="#2C2C34" stroke-width="1.0" filter="url(#mqn_vu_bezel_shadow)"/>'
    )
    res.append(
        '<rect x="3" y="3" width="194" height="126" rx="13.5" '
        'fill="#070709" stroke="#000000" stroke-width="1.2"/>'
    )

    # 2. Interior Aperture Clip Path (strictly clips scale, needle, and pivot inside the 186x118 well)
    res.append(
        f'<clipPath id="{vu_id}_well_clip">'
        f'<rect x="7" y="7" width="186" height="118" rx="10.5"/>'
        f'</clipPath>'
    )

    # 3. Deep Recessed Interior Cavity (BEHIND the glass)
    res.append(f'<g id="{vu_id}_interior_cavity" clip-path="url(#{vu_id}_well_clip)">')
    res.append('<rect x="7" y="7" width="186" height="118" rx="10.5" fill="url(#mqn_vu_cavity_grad)"/>')

    # 4. Vivid Backlit Connected Tick Arc & 9 Calibrated Values (recessed inside cavity)
    res.append(f'  <g id="{vu_id}_tick_arc">')
    # 4a. Subtle Ambient Backlight Whisper behind the Tick Arch
    res.append(
        f'    <path id="{vu_id}_backlight_safe" d="M {x_s:.2f} {y_s:.2f} A {R_arc} {R_arc} 0 0 1 {x_m:.2f} {y_m:.2f}" '
        f'fill="none" stroke="{ACCENT_MAGENTA}" stroke-width="4.0" stroke-opacity="0.14" stroke-linecap="round" filter="url(#mqn_vu_arch_backlight)"/>'
    )
    res.append(
        f'    <path id="{vu_id}_backlight_overload" d="M {x_m:.2f} {y_m:.2f} A {R_arc} {R_arc} 0 0 1 {x_e:.2f} {y_e:.2f}" '
        f'fill="none" stroke="#FF007F" stroke-width="5.5" stroke-opacity="0.28" stroke-linecap="round" filter="url(#mqn_vu_arch_backlight)"/>'
    )
    # 4b. Vivid Backlit Scale Arcs
    res.append(
        f'    <path id="{vu_id}_arc_safe" d="M {x_s:.2f} {y_s:.2f} A {R_arc} {R_arc} 0 0 1 {x_m:.2f} {y_m:.2f}" '
        f'fill="none" stroke="#F8DCE9" stroke-width="1.35" stroke-opacity="0.88" stroke-linecap="round"/>'
    )
    res.append(
        f'    <path id="{vu_id}_arc_overload" d="M {x_m:.2f} {y_m:.2f} A {R_arc} {R_arc} 0 0 1 {x_e:.2f} {y_e:.2f}" '
        f'fill="none" stroke="#FF1E8E" stroke-width="2.2" stroke-linecap="round" opacity="1.0" filter="url(#mqn_vu_subtle_bloom)"/>'
    )

    for deg, val, is_major, is_magenta in tick_specs:
        length = 8.5 if is_major else 5.5
        x_out, y_out = pt(deg, R_arc)
        x_in, y_in = pt(deg, R_arc - length)
        t_stroke = "#FF1E8E" if is_magenta else "#F8DCE9"
        t_op = "1.0" if is_magenta else ("0.92" if is_major else "0.68")
        t_fill = "#FF2491" if is_magenta else "#F2C8DC"

        res.append(
            f'    <line id="{vu_id}_tick_{deg}" data-deg="{deg}" data-val="{val}" '
            f'x1="{x_out:.2f}" y1="{y_out:.2f}" x2="{x_in:.2f}" y2="{y_in:.2f}" '
            f'stroke="{t_stroke}" stroke-width="1.4" stroke-linecap="round" stroke-opacity="{t_op}"/>'
        )
        lx, ly = pt(deg, R_label)
        res.append(
            f'    <text id="{vu_id}_val_{deg}" x="{lx:.2f}" y="{ly:.2f}" '
            f'font-family="\'SF Mono\', Consolas, Menlo, monospace" font-variant-numeric="tabular-nums" '
            f'font-size="6.8" font-weight="700" text-anchor="middle" dominant-baseline="central" '
            f'fill="{t_fill}" opacity="{t_op}">{val}</text>'
        )

    # Classic VU Emblem & Channel Title recessed on backplate
    res.append(
        f'    <text x="{pivot_x:.1f}" y="82" '
        f'font-family="-apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, sans-serif" '
        f'font-size="11" font-weight="800" letter-spacing="1.5" text-anchor="middle" fill="#D484AC" opacity="0.80">VU</text>'
    )
    res.append(
        f'    <text x="{pivot_x:.1f}" y="94" '
        f'font-family="-apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, monospace" '
        f'font-size="7.5" font-weight="700" letter-spacing="1.8" text-anchor="middle" fill="#FF2491" opacity="1.0">{vtitle}</text>'
    )
    res.append('  </g>')

    # 5. Needle Rotor & Pivot Hub (recessed inside cavity, BEHIND the front glass lens)
    res.append('  <g filter="url(#mqn_vu_needle_shadow)">')
    res.append(f'    <g id="{vu_id}_needle" transform="rotate({needle_deg:.1f} {pivot_x:.1f} {pivot_y:.1f})">')
    res.append(
        f'      <rect x="{pivot_x - 1.4:.1f}" y="28" width="2.8" height="90" rx="1.4" '
        f'fill="url(#mqn_vu_needle_grad)"/>'
    )
    res.append('    </g>')
    res.append('  </g>')
    res.append(
        f'  <circle cx="{pivot_x:.1f}" cy="{pivot_y + 1.0:.1f}" r="8.0" '
        f'fill="url(#mqn_vu_pivot_grad)" stroke="#080306" stroke-width="0.9"/>'
    )

    # Deep perimeter cavity shadow cast onto the dial face and needle
    res.append(
        '  <rect x="7.5" y="7.5" width="185" height="117" rx="10" '
        'fill="none" stroke="#000000" stroke-width="3.2" stroke-opacity="0.72" pointer-events="none"/>'
    )
    res.append('</g>')

    # 6. FRONT SMOKED MAGENTA-TINTED OPTICAL GLASS LENS (Sits ON TOP OF needle, scale, and text!)
    res.append(f'<g id="{vu_id}_front_glass_lens" pointer-events="none">')
    # 6a. Smoked Magenta Glass Tint Layer over the entire well
    res.append(
        '<rect x="7" y="7" width="186" height="118" rx="10.5" '
        'fill="url(#mqn_vu_front_glass_tint)"/>'
    )
    # 6b. Clean Top Glass Glare Pill (matching the exact top glare of the segmented displays)
    res.append(
        '<rect x="9.5" y="9.0" width="181" height="10.0" rx="5.0" '
        'fill="#FFFFFF" opacity="0.05"/>'
    )
    # 6c. Glowing Magenta Optical Glass Rim & Inner Bezel Edge
    res.append(
        f'<rect x="7" y="7" width="186" height="118" rx="10.5" '
        f'fill="none" stroke="{ACCENT_MAGENTA}" stroke-opacity="0.26" stroke-width="0.9"/>'
    )
    res.append('</g>')

    res.append('</g>')
    return "\n".join(res)


def draw_segmented_selector(id_name, cx, cy, label, alias_text):
    """
    Consolidated 9-Cell 14-Segment Alphanumeric Starburst Hardware Display + Up/Down Steppers.
    Uses `draw_consolidated_segmented_display` so it shares the exact smoked-glass ghosting aesthetic
    and 12.0px cell pitch with the 6-cell readout labels.
    """
    num_cells = 9
    disp_w = 122.0
    disp_h = 32.0
    btn_w = 24.0
    btn_h = 14.5
    gap = 5.0
    total_w = disp_w + gap + btn_w

    left_x = cx - total_w / 2.0
    disp_cx = left_x + disp_w / 2.0
    disp_y = cy - disp_h / 2.0
    btn_x = left_x + disp_w + gap

    res = [f'<!-- CONSOLIDATED 9-CELL 14-SEGMENT SELECTOR: {id_name} -->']
    res.append(f'<g id="stepper_{id_name}" data-param="{id_name}" cx="{cx}" cy="{cy}">')
    res.append(f'<text x="{cx}" y="{cy - 23}" class="t silm" font-size="10" font-weight="700" letter-spacing="1.5">{label}</text>')

    res.append(
        draw_consolidated_segmented_display(
            id_name=id_name,
            cx=disp_cx,
            cy=cy,
            width=disp_w,
            height=disp_h,
            num_cells=num_cells,
            text_str=alias_text,
            momentary_val=alias_text,
            char_id_prefix="txt",
            rx_outer=8.0,
        )
    )

    # Tactile Up Button [▲]
    res.append(f'''<g id="btn_{id_name}_up" style="cursor: pointer;">
      <rect x="{btn_x:.1f}" y="{disp_y:.1f}" width="{btn_w}" height="{btn_h}" rx="3.5" fill="url(#mqn_rotor_main_grad)" stroke="#2C2C34" stroke-width="0.9"/>
      <path d="M {btn_x + btn_w/2 - 3.5:.1f} {disp_y + btn_h/2 + 2:.1f} L {btn_x + btn_w/2:.1f} {disp_y + btn_h/2 - 2.5:.1f} L {btn_x + btn_w/2 + 3.5:.1f} {disp_y + btn_h/2 + 2:.1f} Z" fill="#ECE9E1"/>
    </g>''')

    # Tactile Down Button [▼]
    res.append(f'''<g id="btn_{id_name}_down" style="cursor: pointer;">
      <rect x="{btn_x:.1f}" y="{disp_y + btn_h + 3:.1f}" width="{btn_w}" height="{btn_h}" rx="3.5" fill="url(#mqn_rotor_main_grad)" stroke="#2C2C34" stroke-width="0.9"/>
      <path d="M {btn_x + btn_w/2 - 3.5:.1f} {disp_y + btn_h + 3 + btn_h/2 - 2:.1f} L {btn_x + btn_w/2:.1f} {disp_y + btn_h + 3 + btn_h/2 + 2.5:.1f} L {btn_x + btn_w/2 + 3.5:.1f} {disp_y + btn_h + 3 + btn_h/2 - 2:.1f} Z" fill="#ECE9E1"/>
    </g>''')

    res.append('</g>')
    return "\n".join(res)


def draw_lufs_display(id_name, x, y, w=170, h=76, lufs_val="-14.2", peak_val="-0.8"):
    """Digital Loudness (INT LUFS / TRUE PK) using two consolidated 5-cell 14-segment displays."""
    lufs_readout = draw_consolidated_segmented_display(
        id_name=f"{id_name}_lufs",
        cx=x + w * 0.26,
        cy=y + 46,
        width=74.0,
        height=30.0,
        num_cells=5,
        text_str=lufs_val,
        momentary_val=lufs_val,
        char_id_prefix="seg_char",
        rx_outer=7.0,
    )
    peak_readout = draw_consolidated_segmented_display(
        id_name=f"{id_name}_peak",
        cx=x + w * 0.74,
        cy=y + 46,
        width=74.0,
        height=30.0,
        num_cells=5,
        text_str=peak_val,
        momentary_val=peak_val,
        char_id_prefix="seg_char",
        rx_outer=7.0,
    )
    return f'''<!-- CONSOLIDATED LOUDNESS DISPLAY: {id_name} -->
    <g id="{id_name}">
      <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="url(#mqn_subpanel_grad)" stroke="#26262C" stroke-width="1.2"/>
      <text x="{x + w*0.26:.1f}" y="{y + 16}" class="t silm" font-size="9.0" font-weight="700" letter-spacing="1.4">INT LUFS</text>
      <text x="{x + w*0.74:.1f}" y="{y + 16}" class="t silm" font-size="9.0" font-weight="700" letter-spacing="1.4">TRUE PK</text>
      {lufs_readout}
      {peak_readout}
    </g>'''


def draw_input_ladder(
    id_name,
    x,
    y,
    w=84.0,
    h=124.0,
    with_top_readout=True,
    readout_label="IN LVL",
    readout_val="-0.8DB",
    lit_l=3,
    lit_r=2,
    is_dark=True,
):
    """
    Upstream Compact Smoked-Glass Stereo LED Bar dB Meter (`84x124` ladder, or `84x160` with top 6-cell readout):
    - Uses the exact same 4-layer smoked magenta-tinted glass bezel construction as the consolidated 14-segment displays.
    - Compacted 84px width (zero wasted horizontal space) aligned flush with the 84x32 top 6-cell segmented readout.
    - 12 stereo LED segments per channel with `#FF007F` ghost cavities (`opacity=0.16`) and `#dseg_bloom` active glow.
    """
    res = [f'<!-- UPSTREAM SMOKED-GLASS STEREO LED DB METER: {id_name} -->']
    res.append(f'<g id="{id_name}" data-param="{id_name}" data-lit-l="{lit_l}" data-lit-r="{lit_r}">')

    ladder_y = y
    if with_top_readout:
        ro_cy = y + 16.0
        res.append(
            draw_consolidated_segmented_display(
                id_name=f"{id_name}_ro",
                cx=x + w / 2.0,
                cy=ro_cy,
                width=w,
                height=32.0,
                num_cells=6,
                text_str=readout_label,
                momentary_val=readout_val,
                char_id_prefix="seg_char",
                rx_outer=8.0,
                is_dark=is_dark,
            )
        )
        ladder_y = y + 36.0

    outer_fill = "#16161B" if is_dark else "#D0CDC6"
    outer_stroke = "#2C2C34" if is_dark else "#B5B2AA"
    trench_fill = "#070709" if is_dark else "#B8B5AE"
    trench_stroke = "#000000" if is_dark else "#96938C"
    lens_fill = "url(#mqn_smoked_lens_grad)" if is_dark else "#DEDAD2"
    lens_stroke_op = "0.22" if is_dark else "0.28"
    glare_op = "0.05" if is_dark else "0.30"

    res.append(f'  <g id="{id_name}_ladder_well" transform="translate({x:.1f}, {ladder_y:.1f})">')
    # 1. Outer Machined Frame (84x124, rx=8.0)
    res.append(
        f'    <rect x="0" y="0" width="{w:.1f}" height="{h:.1f}" rx="8.0" '
        f'fill="{outer_fill}" stroke="{outer_stroke}" stroke-width="1.0"/>'
    )
    # 2. Deep Recessed Bezel Trench (81x121, rx=6.5)
    res.append(
        f'    <rect x="1.5" y="1.5" width="{w - 3.0:.1f}" height="{h - 3.0:.1f}" rx="6.5" '
        f'fill="{trench_fill}" stroke="{trench_stroke}" stroke-width="1.2"/>'
    )
    # 3. Smoked Magenta-Tinted Glass Lens (78x118, rx=5.0)
    res.append(
        f'    <rect x="3.0" y="3.0" width="{w - 6.0:.1f}" height="{h - 6.0:.1f}" rx="5.0" '
        f'fill="{lens_fill}" stroke="{ACCENT_MAGENTA}" stroke-opacity="{lens_stroke_op}" stroke-width="0.8"/>'
    )
    # 4. Top Optical Glass Glare Reflection
    res.append(
        f'    <rect x="4.0" y="4.0" width="{w - 8.0:.1f}" height="8.0" rx="4.0" '
        f'fill="#FFFFFF" opacity="{glare_op}"/>'
    )

    # 5. 12 Stereo LED Segments (Ghost Grid + Lit Active Segments)
    num_segs = 12
    top_pad = 11.5
    bot_pad = 19.5
    usable_h = h - top_pad - bot_pad
    step_y = usable_h / num_segs
    seg_h = step_y - 2.3

    ghost_col = ACCENT_MAGENTA if is_dark else "#B888A0"
    ghost_op = "0.16" if is_dark else "0.26"
    act_filter = ' filter="url(#dseg_bloom)"' if is_dark else ""

    for i in range(num_segs):
        sy = top_pad + i * step_y
        is_overload = i < 2
        is_lit_l = i >= lit_l
        is_lit_r = i >= lit_r
        lit_col = "#FF2491" if is_overload else ("#ECE9E1" if is_dark else "#2C2C2E")

        # Unlit Ghost Cavities (always visible underneath, matching 14-segment ghost aesthetic)
        res.append(
            f'    <rect x="12.5" y="{sy:.2f}" width="16.5" height="{seg_h:.2f}" rx="1.4" '
            f'fill="{ghost_col}" opacity="{ghost_op}"/>'
        )
        res.append(
            f'    <rect x="33.0" y="{sy:.2f}" width="16.5" height="{seg_h:.2f}" rx="1.4" '
            f'fill="{ghost_col}" opacity="{ghost_op}"/>'
        )

        # Active Lit Segments
        op_l = "0.96" if is_lit_l else "0.0"
        op_r = "0.96" if is_lit_r else "0.0"
        res.append(
            f'    <rect id="{id_name}_seg_l_{i}" x="12.5" y="{sy:.2f}" width="16.5" height="{seg_h:.2f}" rx="1.4" '
            f'fill="{lit_col}" opacity="{op_l}"{act_filter if is_overload else ""}/>'
        )
        res.append(
            f'    <rect id="{id_name}_seg_r_{i}" x="33.0" y="{sy:.2f}" width="16.5" height="{seg_h:.2f}" rx="1.4" '
            f'fill="{lit_col}" opacity="{op_r}"{act_filter if is_overload else ""}/>'
        )

    # 6. Calibrated dB Scale Ticks & Tabular Labels (aligned to segment centers)
    scale_marks = [
        (0, "+3", True),
        (2, "0", False),
        (4, "-3", False),
        (6, "-6", False),
        (8, "-12", False),
        (9, "-18", False),
        (11, "-36", False),
    ]
    lbl_safe_col = "#9A9AA4" if is_dark else "#5A5752"
    tick_col = "#686872" if is_dark else "#8E8A82"
    for seg_idx, txt, is_mag in scale_marks:
        cy_seg = top_pad + seg_idx * step_y + seg_h / 2.0
        col = "#FF2491" if is_mag else lbl_safe_col
        t_col = ACCENT_MAGENTA if is_mag else tick_col
        res.append(
            f'    <line x1="51.5" y1="{cy_seg:.2f}" x2="54.5" y2="{cy_seg:.2f}" '
            f'stroke="{t_col}" stroke-width="1.0" stroke-linecap="round" opacity="0.75"/>'
        )
        res.append(
            f'    <text x="66.5" y="{cy_seg:.2f}" '
            f'font-family="\'SF Mono\', Consolas, Menlo, monospace" font-variant-numeric="tabular-nums" '
            f'font-size="7.6" font-weight="700" text-anchor="middle" dominant-baseline="central" '
            f'fill="{col}">{txt}</text>'
        )

    # 7. Bottom L / R Channel Indicators
    ch_y = h - 10.0
    res.append(
        f'    <text x="20.75" y="{ch_y:.1f}" '
        f'font-family="-apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, monospace" '
        f'font-size="7.5" font-weight="800" text-anchor="middle" dominant-baseline="central" '
        f'fill="{lbl_safe_col}">L</text>'
    )
    res.append(
        f'    <text x="41.25" y="{ch_y:.1f}" '
        f'font-family="-apple-system, BlinkMacSystemFont, \'Segoe UI\', Roboto, monospace" '
        f'font-size="7.5" font-weight="800" text-anchor="middle" dominant-baseline="central" '
        f'fill="{lbl_safe_col}">R</text>'
    )
    res.append('  </g>')
    res.append('</g>')
    return "\n".join(res)
