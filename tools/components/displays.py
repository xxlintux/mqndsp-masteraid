"""
Masteraid MQN Tactile Suite — Upstream Reusable 14-Segment Hardware Display Module
Single source of truth for all 14-segment alphanumeric starburst displays across:
- Standalone MQN Suite SVG assets (`assets/mqn_suite` & `assets/knobwork_tactile_suite`)
- Big Knob 6-Cell Momentary/Static Readout Labels (`84x32`, `rx=8`)
- Coloration Cassette 9-Cell Alphanumeric Selectors (`122x32` well + Up/Down steppers)
- Master Output 5-Cell Digital Loudness Readouts (`74x30`, `INT LUFS` / `TRUE PK`)

Key Hardware Accuracy Features:
- Every `<text class="dseg14-active">` element holds at most ONE base character (exact 816-unit monospaced advance width),
  while each cell has its own dedicated hardware decimal point LED (`<circle id="seg_dot_..."/>`) at the bottom-right corner.
- Value strings ending in `DB` or `HZ` lock the unit (`DB` / `HZ`) into the rightmost two cells (Cells 4 & 5) and right-align
  the numerical value in Cells 0..3 so `DB`, `HZ`, and the decimal point never shift horizontally when sweeping a knob.
"""

import base64
import os
import xml.etree.ElementTree as ET

ACCENT_MAGENTA = "#FF007F"
FONT_FILE = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "..", "assets", "fonts", "dseg14-classic-italic.woff2")
)

with open(FONT_FILE, "rb") as _f:
    FONT_B64 = base64.b64encode(_f.read()).decode("ascii")


def _split_chars_and_dots(raw_str):
    """Splits a string into a list of `(char, has_dot)` tuples."""
    items = []
    for ch in raw_str:
        if ch == "." and items and not items[-1][1]:
            items[-1] = (items[-1][0], True)
        else:
            items.append((ch, False))
    return items


def tokenize_dseg_cells(text_str, num_cells):
    """
    Tokenizes `text_str` into exactly `num_cells` tuples of `(base_char, has_dot)`:
    - For 6-cell values ending in `DB`: locks `'D'`, `'B'` into cells 4 & 5 and right-aligns the number in cells 0..3.
    - For 6-cell values ending in `HZ`: locks `'H'`, `'Z'` into cells 4 & 5 and right-aligns the frequency in cells 0..3.
    - For static labels and general strings: centers the tokens across `num_cells`.
    """
    s = str(text_str).upper().strip()

    if num_cells == 6 and s.endswith("DB") and any(c.isdigit() for c in s):
        num_part = s[:-2].strip()
        num_items = _split_chars_and_dots(num_part)
        if len(num_items) < 4:
            num_items = [(" ", False)] * (4 - len(num_items)) + num_items
        else:
            num_items = num_items[-4:]
        return num_items + [("D", False), ("B", False)]

    if num_cells == 6 and s.endswith("HZ") and any(c.isdigit() for c in s):
        freq_part = s[:-2].strip()
        freq_items = _split_chars_and_dots(freq_part)
        if len(freq_items) < 4:
            freq_items = [(" ", False)] * (4 - len(freq_items)) + freq_items
        else:
            freq_items = freq_items[-4:]
        return freq_items + [("H", False), ("Z", False)]

    items = _split_chars_and_dots(s)
    if len(items) < num_cells:
        pad_total = num_cells - len(items)
        pad_left = pad_total // 2
        pad_right = pad_total - pad_left
        items = ([(" ", False)] * pad_left) + items + ([(" ", False)] * pad_right)
    else:
        items = items[:num_cells]
    return items


def get_display_defs(is_dark=True):
    """Returns SVG <defs> (gradients & VFD/LED bloom filter) for the consolidated 14-segment display."""
    lens_top = "#16060E" if is_dark else "#E2DED6"
    lens_bot = "#0D0308" if is_dark else "#D4D0C8"
    return f'''
  <!-- UPSTREAM 14-SEGMENT DISPLAY SMOKED GLASS LENS GRADIENT -->
  <linearGradient id="mqn_smoked_lens_grad" x1="0" y1="0" x2="0" y2="1">
    <stop offset="0%" stop-color="{lens_top}"/>
    <stop offset="100%" stop-color="{lens_bot}"/>
  </linearGradient>

  <!-- UPSTREAM 14-SEGMENT HARDWARE VFD/LED BLOOM FILTER -->
  <filter id="dseg_bloom" x="-30%" y="-30%" width="160%" height="160%">
    <feGaussianBlur in="SourceGraphic" stdDeviation="2.2" result="soft_bloom"/>
    <feColorMatrix in="soft_bloom" type="matrix" values="
      1 0 0 0 0
      0 1 0 0 0
      0 0 1 0 0
      0 0 0 0.75 0" result="tinted_bloom"/>
    <feMerge>
      <feMergeNode in="tinted_bloom"/>
      <feMergeNode in="SourceGraphic"/>
    </feMerge>
  </filter>
'''


def get_display_css(is_dark=True, include_font_face=True):
    """Returns the canonical CSS rules for `.dseg14-ghost`, `.dseg14-active`, and `.dseg14-dot`."""
    ghost_fill = ACCENT_MAGENTA if is_dark else "#B888A0"
    ghost_opacity = "0.16" if is_dark else "0.30"
    act_fill = "#FF2491" if is_dark else ACCENT_MAGENTA
    act_filter = "filter: url(#dseg_bloom);" if is_dark else ""

    font_face_block = ""
    if include_font_face:
        font_face_block = f'''
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
    }}'''

    return f'''{font_face_block}
    .dseg14-ghost {{
      font-family: 'DSEG14 Classic', 'DSEG14 Classic Italic', monospace;
      font-style: normal;
      font-synthesis: none;
      font-size: 13.8px;
      font-weight: 700;
      fill: {ghost_fill};
      opacity: {ghost_opacity};
      text-anchor: middle;
      dominant-baseline: central;
    }}
    .dseg14-active {{
      font-family: 'DSEG14 Classic', 'DSEG14 Classic Italic', monospace;
      font-style: normal;
      font-synthesis: none;
      font-size: 13.8px;
      font-weight: 700;
      fill: {act_fill};
      text-anchor: middle;
      dominant-baseline: central;
      {act_filter}
    }}
    .dseg14-dot {{
      fill: {act_fill};
      {act_filter}
    }}'''


def draw_consolidated_segmented_display(
    id_name,
    cx,
    cy,
    width=84.0,
    height=32.0,
    num_cells=6,
    text_str="TREBLE",
    momentary_val="+0.0DB",
    char_id_prefix="seg_char",
    rx_outer=8.0,
    is_dark=True,
):
    """
    Upstream Consolidated 14-Segment Alphanumeric Starburst Hardware Display:
    - Fixed 12.0px monospaced character grid where each cell has:
      1. `<text class="dseg14-ghost">~</text>` (unlit 14-segment starburst)
      2. `<text class="dseg14-active">` (single base character with uniform 816-unit advance width)
      3. `<circle class="dseg14-dot">` (dedicated hardware decimal point LED at bottom-right of cell)
    """
    left_x = cx - width / 2.0
    top_y = cy - height / 2.0
    static_str = str(text_str).upper().strip()
    tokens = tokenize_dseg_cells(static_str, num_cells)

    pitch = 12.0
    total_grid_w = (num_cells - 1) * pitch
    start_x = cx - total_grid_w / 2.0
    mid_y = cy + 0.5

    cells = []
    for i in range(num_cells):
        cell_x = start_x + i * pitch
        base_ch, has_dot = tokens[i]
        disp_ch = "" if base_ch == " " else base_ch
        dot_op = "1.0" if has_dot else "0.0"

        if char_id_prefix == "seg_char":
            cid = f"seg_char_{id_name}_{i}" if id_name else f"seg_char_{i}"
            did = f"seg_dot_{id_name}_{i}" if id_name else f"seg_dot_{i}"
            cell_gid = f"seg_cell_{id_name}_{i}" if id_name else f"seg_cell_{i}"
        else:
            cid = f"{char_id_prefix}_{id_name}_c{i}"
            did = f"{char_id_prefix}_dot_{id_name}_c{i}"
            cell_gid = f"seg_cell_{id_name}_{i}"

        cells.append(
            f'<g id="{cell_gid}" class="dseg14-cell">'
            f'<text x="{cell_x:.1f}" y="{mid_y:.1f}" class="dseg14-ghost">~</text>'
            f'<text id="{cid}" x="{cell_x:.1f}" y="{mid_y:.1f}" class="dseg14-active">{disp_ch}</text>'
            f'<circle id="{did}" cx="{cell_x + 5.4:.1f}" cy="{mid_y + 6.2:.1f}" r="1.15" class="dseg14-dot" opacity="{dot_op}"/>'
            f'</g>'
        )
    cells_svg = "\n      ".join(cells)

    rx_mid = max(2.0, rx_outer - 1.5)
    rx_lens = max(1.5, rx_outer - 3.0)
    rx_glare = max(1.0, rx_outer - 4.0)

    outer_fill = "#16161B" if is_dark else "#D0CDC6"
    outer_stroke = "#2C2C34" if is_dark else "#B5B2AA"
    trench_fill = "#070709" if is_dark else "#B8B5AE"
    trench_stroke = "#000000" if is_dark else "#96938C"
    lens_fill = "url(#mqn_smoked_lens_grad)" if is_dark else "#DEDAD2"
    lens_stroke_op = "0.22" if is_dark else "0.28"
    glare_op = "0.05" if is_dark else "0.30"
    box_id = f"readout_box_{id_name}" if id_name else "readout_display"

    return f'''<!-- CONSOLIDATED {num_cells}-CELL 14-SEGMENT DISPLAY: {id_name or "standalone"} -->
    <g id="{box_id}" data-role="segmented-display" data-param="{id_name}" data-static="{static_str}" data-momentary="{momentary_val}" data-mode="static" data-cells="{num_cells}">
      <!-- 1. Outer Machined Readout Label Frame -->
      <rect x="{left_x:.1f}" y="{top_y:.1f}" width="{width:.1f}" height="{height:.1f}" rx="{rx_outer:.1f}" fill="{outer_fill}" stroke="{outer_stroke}" stroke-width="1.0"/>
      <!-- 2. Deep Recessed Bezel Trench -->
      <rect x="{left_x + 1.5:.1f}" y="{top_y + 1.5:.1f}" width="{width - 3.0:.1f}" height="{height - 3.0:.1f}" rx="{rx_mid:.1f}" fill="{trench_fill}" stroke="{trench_stroke}" stroke-width="1.2"/>
      <!-- 3. Smoked Tinted Glass Lens with Subtle Glowing Inner Rim -->
      <rect x="{left_x + 3.0:.1f}" y="{top_y + 3.0:.1f}" width="{width - 6.0:.1f}" height="{height - 6.0:.1f}" rx="{rx_lens:.1f}" fill="{lens_fill}" stroke="{ACCENT_MAGENTA}" stroke-opacity="{lens_stroke_op}" stroke-width="0.8"/>
      <!-- 4. Optical Glass Top Glare Reflection -->
      <rect x="{left_x + 4.0:.1f}" y="{top_y + 4.0:.1f}" width="{width - 8.0:.1f}" height="7.0" rx="{rx_glare:.1f}" fill="#FFFFFF" opacity="{glare_op}"/>
      <!-- 5. Fixed {num_cells}-Cell 14-Segment Starburst Grid + Active Characters -->
      {cells_svg}
    </g>'''


def build_consolidated_display_element(
    is_dark=True,
    cx=80.0,
    cy=168.0,
    width=84.0,
    height=32.0,
    num_cells=6,
    label="TREBLE",
    momentary="+0.0DB",
    rx_outer=8.0,
):
    """
    Returns an `xml.etree.ElementTree.Element` for `build_knobwork_suite_matrix.py` and standalone SVG asset builders.
    """
    svg_fragment = draw_consolidated_segmented_display(
        id_name="",
        cx=cx,
        cy=cy,
        width=width,
        height=height,
        num_cells=num_cells,
        text_str=label,
        momentary_val=momentary,
        char_id_prefix="seg_char",
        rx_outer=rx_outer,
        is_dark=is_dark,
    )
    return ET.fromstring(svg_fragment)
