"""
Masteraid MQN Tactile Suite — Upstream Illuminated Smoked-Glass Button & Switch Components
- Squared Smoked-Glass Push Button (`44x44`, matching the consolidated 14-segment display bezel, recessed trench,
  magenta-tinted smoked glass lens when OFF, backlit neon `#FF007F` bloom when ON, and top optical glass glare).
- Upstream 1176-Style Multi-Button Ratio Selector Bank (`4`, `8`, `12`, `20`, horizontal or vertical).
- MQN Illuminated Bypass Button ('IN' with #FF007F LED indicator).
- MQN Two-Way Model Selector Toggle.
"""

ACCENT_MAGENTA = "#FF007F"


def get_button_defs(is_dark=True):
    """Returns SVG <defs> for the Squared Smoked-Glass Push Buttons (OFF, ON, and SMASHED states)."""
    lens_top = "#210715" if is_dark else "#E2DED6"
    lens_mid = "#14040C" if is_dark else "#DAD6CE"
    lens_bot = "#0B0207" if is_dark else "#D4D0C8"
    return f'''
  <!-- SMOKED GLASS PUSH BUTTON UNLIT (OFF) TINTED LENS GRADIENT -->
  <linearGradient id="mqn_btn_smoked_off_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="{lens_top}"/>
    <stop offset="55%" stop-color="{lens_mid}"/>
    <stop offset="100%" stop-color="{lens_bot}"/>
  </linearGradient>

  <!-- SMOKED GLASS CHAMFERED BEVEL RIM GRADIENT (EDGE-LIT IDLE STATE) -->
  <linearGradient id="mqn_btn_bevel_rim_grad" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#FF80BF" stop-opacity="0.78"/>
    <stop offset="45%" stop-color="#FF007F" stop-opacity="0.45"/>
    <stop offset="100%" stop-color="#FF3399" stop-opacity="0.72"/>
  </linearGradient>

  <!-- SMOKED GLASS CHAMFERED BEVEL RIM GRADIENT (BACKLIT ON STATE) -->
  <linearGradient id="mqn_btn_bevel_rim_on_grad" x1="0" y1="0" x2="1" y2="1">
    <stop offset="0%" stop-color="#FFD6EB" stop-opacity="0.98"/>
    <stop offset="50%" stop-color="#FF1E8E" stop-opacity="0.92"/>
    <stop offset="100%" stop-color="#FF80BF" stop-opacity="0.98"/>
  </linearGradient>

  <!-- SMOKED GLASS PUSH BUTTON LIT (ON) BACKLIT MAGENTA LENS GRADIENT -->
  <radialGradient id="mqn_btn_smoked_on_grad" cx="50%" cy="45%" r="65%">
    <stop offset="0%" stop-color="#7A0C43"/>
    <stop offset="58%" stop-color="#3D0622"/>
    <stop offset="100%" stop-color="#1E0310"/>
  </radialGradient>

  <!-- SMOKED GLASS PUSH BUTTON SMASHED (EXTREME DRIVE) OVERDRIVEN CORE GRADIENT -->
  <radialGradient id="mqn_btn_smoked_smashed_grad" cx="56%" cy="42%" r="68%">
    <stop offset="0%" stop-color="#FF99CC"/>
    <stop offset="28%" stop-color="#D4006A"/>
    <stop offset="68%" stop-color="#630436"/>
    <stop offset="100%" stop-color="#260214"/>
  </radialGradient>
'''


def get_button_css(is_dark=True):
    """Returns the canonical CSS classes for `.btn-amber-lit`, `.btn-amber-unlit`, `.btn-amber-smashed`, bevels, and text states."""
    unlit_fill = "url(#mqn_btn_smoked_off_grad)" if is_dark else "#DEDAD2"
    unlit_stroke_op = "0.62" if is_dark else "0.42"
    unlit_txt = "#D478A8" if is_dark else "#7A5268"
    return f'''
    .btn-amber-lit {{
      fill: url(#mqn_btn_smoked_on_grad);
      stroke: #FF4DA6;
      stroke-opacity: 0.96;
      stroke-width: 1.2px;
    }}
    .btn-amber-smashed {{
      fill: url(#mqn_btn_smoked_smashed_grad);
      stroke: #FFB3D9;
      stroke-opacity: 1.0;
      stroke-width: 1.35px;
    }}
    .btn-amber-unlit {{
      fill: {unlit_fill};
      stroke: #FF2491;
      stroke-opacity: {unlit_stroke_op};
      stroke-width: 1.05px;
    }}
    .btn-bevel-unlit {{
      fill: none;
      stroke: url(#mqn_btn_bevel_rim_grad);
      stroke-width: 1.15px;
      opacity: 0.72;
    }}
    .btn-bevel-lit {{
      fill: none;
      stroke: url(#mqn_btn_bevel_rim_on_grad);
      stroke-width: 1.35px;
      opacity: 0.96;
      filter: url(#dseg_bloom);
    }}
    .btn-bevel-smashed {{
      fill: none;
      stroke: #FFE6F2;
      stroke-width: 1.5px;
      opacity: 1.0;
      filter: url(#dseg_bloom);
    }}
    .btn-text-lit {{
      fill: #FF66B2;
      font-weight: 800;
      letter-spacing: 1.2px;
      font-size: 9.5px;
      filter: url(#dseg_bloom);
    }}
    .btn-text-smashed {{
      fill: #FFF0F7;
      font-weight: 800;
      letter-spacing: 1.2px;
      font-size: 9.5px;
      filter: url(#dseg_bloom);
    }}
    .btn-text-unlit {{
      fill: {unlit_txt};
      opacity: 0.84;
      font-weight: 800;
      letter-spacing: 1.2px;
      font-size: 9.5px;
    }}
'''


def draw_vintage_push_button(
    id_name,
    cx,
    cy,
    label="PUSH",
    is_lit=False,
    state=None,
    width=44,
    height=44,
    is_dark=True,
):
    """
    Upstream Squared Smoked-Glass Illuminated Latch Push Button (`44x44` default) with 3 States:
    - `state="off"` (`0`): Smoked glass with edge-lit magenta chamfered glass bevel & legible legend.
    - `state="on"` (`1`): Backlit glowing magenta lens, bright illuminated glass bevel & bloom legend (normal Harmonic Drive).
    - `state="smashed"` (`2`): Overdriven white-hot magenta core with shattered tempered glass
      spiderweb fractures, tilted shard facets, and extreme Harmonic Drive (triggered on press-and-hold).
    """
    if state is None:
        state = "on" if is_lit else "off"
    state = str(state).lower()

    w = float(width)
    h = float(height)
    x = cx - w / 2.0
    y = cy - h / 2.0

    if state == "smashed":
        btn_lens_cls = "btn-amber-smashed"
        btn_bevel_cls = "btn-bevel-smashed"
        btn_txt_cls = "btn-text-smashed"
        lit_attr = "2"
        smash_op = "1"
        glare_op = "0.03"
    elif state == "on":
        btn_lens_cls = "btn-amber-lit"
        btn_bevel_cls = "btn-bevel-lit"
        btn_txt_cls = "btn-text-lit"
        lit_attr = "1"
        smash_op = "0"
        glare_op = "0.09" if is_dark else "0.30"
    else:
        btn_lens_cls = "btn-amber-unlit"
        btn_bevel_cls = "btn-bevel-unlit"
        btn_txt_cls = "btn-text-unlit"
        lit_attr = "0"
        smash_op = "0"
        glare_op = "0.07" if is_dark else "0.30"

    outer_fill = "#16161B" if is_dark else "#D0CDC6"
    outer_stroke = "#2C2C34" if is_dark else "#B5B2AA"
    trench_fill = "#070709" if is_dark else "#B8B5AE"
    trench_stroke = "#000000" if is_dark else "#96938C"

    # Impact epicenter slightly upper-right of center for organic asymmetric shatter
    ix = cx + 3.5
    iy = cy - 2.5
    lx = x + 3.0
    ly = y + 3.0
    lw = w - 6.0
    lh = h - 6.0
    rx_edge = lx + lw
    by_edge = ly + lh

    res = [f'<!-- UPSTREAM SQUARED SMOKED-GLASS PUSH BUTTON: {id_name} (state={state}) -->']
    res.append(
        f'<g id="btn_{id_name}" data-param="{id_name}" data-type="vintage-push" '
        f'data-lit="{lit_attr}" data-state="{lit_attr}" data-cx="{cx:.1f}" data-cy="{cy:.1f}" style="cursor: pointer;">'
    )

    # Clip path for the 38x38 glass lens so fractures stay strictly inside the recessed bezel
    res.append(
        f'  <clipPath id="lens_clip_{id_name}">'
        f'<rect x="{lx:.1f}" y="{ly:.1f}" width="{lw:.1f}" height="{lh:.1f}" rx="5.0"/>'
        f'</clipPath>'
    )

    # 1. Outer Machined Frame (rx=8.0)
    res.append(
        f'  <rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="8.0" '
        f'fill="{outer_fill}" stroke="{outer_stroke}" stroke-width="1.0"/>'
    )
    # 2. Deep Recessed Bezel Trench (1.5px inset, rx=6.5)
    res.append(
        f'  <rect x="{x + 1.5:.1f}" y="{y + 1.5:.1f}" width="{w - 3.0:.1f}" height="{h - 3.0:.1f}" rx="6.5" '
        f'fill="{trench_fill}" stroke="{trench_stroke}" stroke-width="1.2"/>'
    )
    # 3. Smoked Tinted Glass Cap / Lens (3.0px inset, rx=5.0)
    res.append(
        f'  <rect id="lens_{id_name}" x="{lx:.1f}" y="{ly:.1f}" width="{lw:.1f}" height="{lh:.1f}" '
        f'rx="5.0" class="{btn_lens_cls}"/>'
    )
    # 3b. Illuminated 3D Glass Bevel Rim (4.1px inset, rx=4.0 — catches edge light in all states)
    res.append(
        f'  <rect id="bevel_{id_name}" x="{x + 4.1:.1f}" y="{y + 4.1:.1f}" width="{w - 8.2:.1f}" height="{h - 8.2:.1f}" '
        f'rx="4.0" class="{btn_bevel_cls}" pointer-events="none"/>'
    )
    # 4. Engraved Backlit Legend (sits physically behind the front glass surface & fracture plane)
    res.append(
        f'  <text id="txt_{id_name}" x="{cx:.1f}" y="{cy + 0.5:.1f}" class="t {btn_txt_cls}" pointer-events="none">{label}</text>'
    )
    # 5. Optical Glass Top Glare Reflection (4.5px inset, rx=3.2)
    res.append(
        f'  <rect id="glare_{id_name}" x="{x + 4.5:.1f}" y="{y + 4.5:.1f}" width="{w - 9.0:.1f}" height="7.0" rx="3.2" '
        f'fill="#FFFFFF" opacity="{glare_op}" pointer-events="none"/>'
    )

    # 6. Shattered Tempered-Glass Overlay (Visible in State 2: "smashed" / Press-and-Hold Extreme Drive)
    res.append(
        f'  <g id="smash_{id_name}" clip-path="url(#lens_clip_{id_name})" opacity="{smash_op}" pointer-events="none">'
    )
    # Tilted glass shard facets (light & shadow refraction triangles)
    res.append(
        f'    <polygon points="{ix:.1f},{iy:.1f} {lx + 6:.1f},{ly:.1f} {lx + 22:.1f},{ly:.1f}" fill="#FFFFFF" opacity="0.14"/>'
        f'    <polygon points="{ix:.1f},{iy:.1f} {rx_edge:.1f},{ly + 6:.1f} {rx_edge:.1f},{ly + 24:.1f}" fill="#000000" opacity="0.24"/>'
        f'    <polygon points="{ix:.1f},{iy:.1f} {lx:.1f},{ly + 14:.1f} {lx:.1f},{by_edge - 4:.1f}" fill="#FF80BF" opacity="0.12"/>'
        f'    <polygon points="{ix:.1f},{iy:.1f} {lx + 10:.1f},{by_edge:.1f} {rx_edge - 6:.1f},{by_edge:.1f}" fill="#FFFFFF" opacity="0.09"/>'
    )
    # Inner concentric spiderweb stress rings around impact point
    res.append(
        f'    <polygon points="{ix - 4.2:.1f},{iy - 2.0:.1f} {ix - 0.8:.1f},{iy - 4.8:.1f} {ix + 4.0:.1f},{iy - 2.6:.1f} '
        f'{ix + 4.8:.1f},{iy + 2.2:.1f} {ix + 0.5:.1f},{iy + 5.0:.1f} {ix - 4.5:.1f},{iy + 2.4:.1f}" '
        f'fill="none" stroke="#FFE6F2" stroke-width="0.65" opacity="0.85"/>'
        f'    <polygon points="{ix - 9.5:.1f},{iy - 4.5:.1f} {ix - 2.0:.1f},{iy - 10.2:.1f} {ix + 8.5:.1f},{iy - 6.0:.1f} '
        f'{ix + 10.2:.1f},{iy + 4.8:.1f} {ix + 2.2:.1f},{iy + 11.2:.1f} {ix - 8.8:.1f},{iy + 6.5:.1f}" '
        f'fill="none" stroke="#FF99CC" stroke-width="0.55" opacity="0.72"/>'
    )
    # Dark refraction shadow clefts behind main radial cracks
    res.append(
        f'    <path d="'
        f'M {ix:.1f} {iy:.1f} L {ix - 7:.1f} {iy - 8:.1f} L {lx + 5:.1f} {ly:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 4:.1f} {iy - 9:.1f} L {rx_edge - 7:.1f} {ly:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 9:.1f} {iy - 2:.1f} L {rx_edge:.1f} {ly + 9:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 8:.1f} {iy + 7:.1f} L {rx_edge:.1f} {by_edge - 6:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 1:.1f} {iy + 10:.1f} L {ix + 4:.1f} {by_edge:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 7:.1f} {iy + 8:.1f} L {lx + 6:.1f} {by_edge:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 11:.1f} {iy + 2:.1f} L {lx:.1f} {cy + 4:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 10:.1f} {iy - 3:.1f} L {lx:.1f} {ly + 10:.1f}" '
        f'fill="none" stroke="#14000A" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round" opacity="0.82" transform="translate(0.4, 0.5)"/>'
    )
    # Specular white-magenta razor fracture edges
    res.append(
        f'    <path d="'
        f'M {ix:.1f} {iy:.1f} L {ix - 7:.1f} {iy - 8:.1f} L {lx + 5:.1f} {ly:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 4:.1f} {iy - 9:.1f} L {rx_edge - 7:.1f} {ly:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 9:.1f} {iy - 2:.1f} L {rx_edge:.1f} {ly + 9:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 8:.1f} {iy + 7:.1f} L {rx_edge:.1f} {by_edge - 6:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix + 1:.1f} {iy + 10:.1f} L {ix + 4:.1f} {by_edge:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 7:.1f} {iy + 8:.1f} L {lx + 6:.1f} {by_edge:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 11:.1f} {iy + 2:.1f} L {lx:.1f} {cy + 4:.1f} '
        f'M {ix:.1f} {iy:.1f} L {ix - 10:.1f} {iy - 3:.1f} L {lx:.1f} {ly + 10:.1f}" '
        f'fill="none" stroke="#FFF0F8" stroke-width="0.75" stroke-linecap="round" stroke-linejoin="round" opacity="0.95"/>'
    )
    # Impact point white-hot crushed glass core
    res.append(
        f'    <circle cx="{ix:.1f}" cy="{iy:.1f}" r="1.6" fill="#FFFFFF" opacity="0.95"/>'
    )
    res.append('  </g>')
    res.append('</g>')
    return "\n".join(res)


def draw_1176_ratio_bank(
    id_name,
    cx,
    cy,
    ratios=("4", "8", "12", "20"),
    active_idx=0,
    orientation="horizontal",
    btn_size=44.0,
    gap=6.0,
    is_dark=True,
):
    """
    Upstream 1176-Style Squared Smoked-Glass Ratio Selector Bank (`4`, `8`, `12`, `20`).
    Can be oriented horizontally or vertically.
    """
    n = len(ratios)
    span = n * btn_size + (n - 1) * gap
    res = [f'<!-- UPSTREAM 1176-STYLE SMOKED-GLASS RATIO BANK: {id_name} ({orientation}) -->']
    res.append(f'<g id="ratio_bank_{id_name}" data-param="{id_name}" data-type="ratio-bank">')

    for i, r_lbl in enumerate(ratios):
        offset = -span / 2.0 + btn_size / 2.0 + i * (btn_size + gap)
        bx = (cx + offset) if orientation == "horizontal" else cx
        by = cy if orientation == "horizontal" else (cy + offset)
        res.append(
            draw_vintage_push_button(
                id_name=f"{id_name}_{r_lbl}",
                cx=bx,
                cy=by,
                label=r_lbl,
                is_lit=(i == active_idx),
                width=btn_size,
                height=btn_size,
                is_dark=is_dark,
            )
        )

    res.append('</g>')
    return "\n".join(res)


def draw_vintage_bypass_button(id_name, cx, cy, label="IN", is_on=True):
    """
    MQN Tactile Bypass Switch with #FF007F Magenta LED indicator dot.
    """
    dot_cls = "dot-amber" if is_on else "dot-amber-p"
    txt_cls = "t text-amber-active" if is_on else "t model-inactive"
    return f'''<!-- MQN BYPASS BUTTON: {id_name} -->
    <g id="btn_{id_name}" data-param="{id_name}" data-type="vintage-bypass" data-on="{1 if is_on else 0}" transform="translate({cx:.1f}, {cy:.1f})" style="cursor: pointer;">
      <rect x="-32" y="-13" width="64" height="26" rx="6" fill="#0B0B0D" stroke="#26262C" stroke-width="1.2"/>
      <rect x="-29" y="-10" width="58" height="20" rx="4" fill="url(#mqn_rotor_main_grad)" stroke="#18181C" stroke-width="0.8"/>
      <circle id="dot_{id_name}" cx="-14" cy="0" r="3.5" class="{dot_cls}"/>
      <text id="txt_{id_name}" x="7" y="0" class="{txt_cls}" font-size="10">{label}</text>
    </g>'''


def draw_model_select(id_name, cx, cy, opt_a, opt_b, active="A", width=200):
    """
    MQN Two-Way Sliding Model Selector Pill.
    """
    w = width
    h = 28
    x = cx - w / 2
    y = cy - h / 2
    btn_w = w / 2 - 2
    res = [f'<!-- MQN MODEL SELECT: {id_name} -->', f'<g id="model_select_{id_name}">']
    res.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w}" height="{h}" rx="8" fill="#0B0B0D" stroke="#26262C" stroke-width="1.2"/>')
    if active == "A":
        res.append(f'<rect x="{x+2:.1f}" y="{y+2:.1f}" width="{btn_w:.1f}" height="{h-4}" rx="6" fill="url(#mqn_rotor_main_grad)" stroke="#383840" stroke-width="0.8"/>')
        res.append(f'<text x="{x + w*0.25:.1f}" y="{cy:.1f}" class="t model-active">{opt_a}</text>')
        res.append(f'<text x="{x + w*0.75:.1f}" y="{cy:.1f}" class="t model-inactive">{opt_b}</text>')
    else:
        res.append(f'<rect x="{x+w/2:.1f}" y="{y+2:.1f}" width="{btn_w:.1f}" height="{h-4}" rx="6" fill="url(#mqn_rotor_main_grad)" stroke="#383840" stroke-width="0.8"/>')
        res.append(f'<text x="{x + w*0.25:.1f}" y="{cy:.1f}" class="t model-inactive">{opt_a}</text>')
        res.append(f'<text x="{x + w*0.75:.1f}" y="{cy:.1f}" class="t model-active">{opt_b}</text>')
    res.append('</g>')
    return "\n".join(res)
