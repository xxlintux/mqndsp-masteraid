import base64
import os

def b64(path):
    with open(path, 'rb') as f:
        data = f.read()
    if path.endswith('.svg'):
        return 'data:image/svg+xml;base64,' + base64.b64encode(data).decode()
    elif path.endswith('.png'):
        return 'data:image/png;base64,' + base64.b64encode(data).decode()
    return ''

rust_b64 = b64('assets/visuals/branding/rust_logo.png')
clap_b64 = b64('assets/visuals/branding/clap_logo.svg')
vst3_b64 = b64('assets/visuals/branding/vst3_logo_white.svg')

# Airwindows clean analog wave icon
airwindows_icon = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#ffffff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12c3-9 5-9 8 0s5 9 8 0 3-4 4-4"/></svg>'''
airwindows_b64 = 'data:image/svg+xml;base64,' + base64.b64encode(airwindows_icon.encode()).decode()

# MIT / OSI Clean open source license icon
mit_icon = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#ffffff"><path d="M12 2a10 10 0 0 0-7.07 17.07l2.83-2.83A6 6 0 1 1 12 18a6 6 0 0 1-2.83-.71l-2.83 2.83A10 10 0 1 0 12 2z"/></svg>'''
mit_b64 = 'data:image/svg+xml;base64,' + base64.b64encode(mit_icon.encode()).decode()

def make_badge(logo_b64, label, value, left_bg, right_bg, left_w, right_w):
    total_w = left_w + right_w
    label_x = 22 + (left_w - 22) / 2
    value_x = left_w + right_w / 2
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="{total_w}" height="20" role="img" aria-label="{label}: {value}">
  <g shape-rendering="crispEdges">
    <rect width="{left_w}" height="20" fill="{left_bg}"/>
    <rect x="{left_w}" width="{right_w}" height="20" fill="{right_bg}"/>
  </g>
  <image x="4" y="3" width="14" height="14" href="{logo_b64}"/>
  <g fill="#fff" text-anchor="middle" font-family="-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif" font-size="11" font-weight="500">
    <text x="{label_x}" y="14" fill="#ddd">{label}</text>
    <text x="{value_x}" y="14" font-weight="700">{value}</text>
  </g>
</svg>'''
    return svg

# Linux Tux icon
linux_icon = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#ffffff"><path d="M12 2a4.5 4.5 0 0 0-4.5 4.5c0 1.2.5 2.3 1.2 3.1-.7 1.5-1.7 4.1-1.7 6.4 0 2.2 1.8 4 5 4s5-1.8 5-4c0-2.3-1-4.9-1.7-6.4.7-.8 1.2-1.9 1.2-3.1A4.5 4.5 0 0 0 12 2z"/></svg>'''
linux_b64 = 'data:image/svg+xml;base64,' + base64.b64encode(linux_icon.encode()).decode()

# Windows icon
windows_icon = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#ffffff"><path d="M3 5.48L10.33 4.5v6.52H3V5.48zm0 13.04l7.33.98v-6.52H3v5.54zm8.33 1.1L21 21V11.98h-9.67v7.64zM21 3l-9.67 1.38V11H21V3z"/></svg>'''
windows_b64 = 'data:image/svg+xml;base64,' + base64.b64encode(windows_icon.encode()).decode()

badges = [
    ('badge_rust.svg', rust_b64, 'Built with', 'Rust', '#333333', '#C86432', 76, 44),
    ('badge_clap.svg', clap_b64, 'Format', 'CLAP', '#333333', '#E0004D', 62, 46),
    ('badge_vst3.svg', vst3_b64, 'Format', 'VST3', '#333333', '#0072C6', 62, 46),
    ('badge_airwindows.svg', airwindows_b64, 'DSP', 'Airwindows', '#333333', '#7952B3', 48, 78),
    ('badge_windows.svg', windows_b64, 'OS', 'Windows', '#333333', '#0078D6', 42, 64),
    ('badge_linux.svg', linux_b64, 'OS', 'Linux', '#333333', '#EAA000', 42, 48),
    ('badge_license.svg', mit_b64, 'License', 'MIT', '#333333', '#10B981', 62, 40),
]

for filename, logo, lbl, val, lbg, rbg, lw, rw in badges:
    svg_data = make_badge(logo, lbl, val, lbg, rbg, lw, rw)
    out_path = os.path.join('assets', 'visuals', 'branding', filename)
    with open(out_path, 'w', encoding='utf-8') as f:
        f.write(svg_data)
    print(f'Generated {filename} ({len(svg_data)} bytes)')
