"""
Masteraid Slider / Fader Components
Features:
- Machined brushed aluminum fader cap
- Calibrated scale ticks & track groove
- Fixed physical lighting highlight on the cap
"""

def draw_slider(id_name, cx, cy, label, value=0.5, length=120, orientation="vertical"):
    """
    Analog Mastering Fader / Slider.
    """
    res = [f'<!-- SLIDER: {id_name} -->']
    half_l = length / 2
    res.append(f'<g id="slider_{id_name}" data-param="{id_name}" data-type="slider" cx="{cx}" cy="{cy}">')
    
    # Track slot
    if orientation == "vertical":
        res.append(f'<line x1="{cx}" y1="{cy - half_l}" x2="{cx}" y2="{cy + half_l}" stroke="#0a0c0e" stroke-width="6" stroke-linecap="round"/>')
        res.append(f'<line x1="{cx}" y1="{cy - half_l}" x2="{cx}" y2="{cy + half_l}" stroke="#000000" stroke-width="2" stroke-linecap="round"/>')
        
        # Scale ticks
        for i in range(-5, 6):
            ty = cy + i * (half_l / 5)
            is_zero = (i == 0)
            res.append(f'<line x1="{cx - 16}" y1="{ty}" x2="{cx - 8}" y2="{ty}" stroke="#ece9e1" stroke-opacity="{0.6 if is_zero else 0.2}" stroke-width="{1.5 if is_zero else 0.8}"/>')
            res.append(f'<line x1="{cx + 8}" y1="{ty}" x2="{cx + 16}" y2="{ty}" stroke="#ece9e1" stroke-opacity="{0.6 if is_zero else 0.2}" stroke-width="{1.5 if is_zero else 0.8}"/>')
        
        # Fader Handle Position (value between 0.0 bottom to 1.0 top)
        handle_y = cy + half_l - value * length
        cap_w = 32
        cap_h = 16
        hx = cx - cap_w / 2
        hy = handle_y - cap_h / 2
        
        # Fader Cap Drop Shadow
        res.append(f'<rect x="{hx}" y="{hy + 3}" width="{cap_w}" height="{cap_h}" rx="3" fill="#000000" opacity="0.6" filter="url(#drop)"/>')
        # Brushed Metal Cap Body
        res.append(f'<rect id="handle_{id_name}" x="{hx}" y="{hy}" width="{cap_w}" height="{cap_h}" rx="3" fill="url(#skirt)" stroke="url(#rim)" stroke-width="0.8"/>')
        # Center White Index Line
        res.append(f'<line x1="{cx - 8}" y1="{handle_y}" x2="{cx + 8}" y2="{handle_y}" stroke="#ffffff" stroke-width="1.8"/>')
        # Static Lighting Specular Highlight
        res.append(f'<rect x="{hx+1}" y="{hy+1}" width="{cap_w-2}" height="{cap_h/2}" rx="2" fill="#ffffff" opacity="0.12" pointer-events="none"/>')
        res.append(f'<text x="{cx}" y="{cy + half_l + 18}" class="t silm" font-size="9" letter-spacing="1.5">{label}</text>')
    
    res.append('</g>')
    return "\n".join(res)
