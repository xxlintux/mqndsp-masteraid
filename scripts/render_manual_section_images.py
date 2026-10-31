# -*- coding: utf-8 -*-
import os
from PIL import Image

out_dir = os.path.abspath("assets/visuals/screenshots")
dark_path = os.path.join(out_dir, "masteraid_gui_dark.png")
light_path = os.path.join(out_dir, "masteraid_gui_light.png")

img_dark = Image.open(dark_path)
img_light = Image.open(light_path)
w, h = img_dark.size
print(f"Loaded real GPU screenshots: {w}x{h}")

# 1. Left Wing: Input Stage & Filters
sec02 = img_dark.crop((24, 134, 244, 866))
sec02.save(os.path.join(out_dir, "sec02_input_filters.png"))
print("Saved sec02_input_filters.png:", sec02.size)

# 2. Master EQ (MASS mode from default dark)
sec03_mass = img_dark.crop((240, 134, 1120, 502))
sec03_mass.save(os.path.join(out_dir, "sec03_eq_mass.png"))
sec03_mass.save(os.path.join(out_dir, "sec03_master_eq.png"))
print("Saved sec03_eq_mass.png:", sec03_mass.size)

# 3. Master Dynamics (FAIR mode from default dark)
sec04_fair = img_dark.crop((240, 496, 1120, 866))
sec04_fair.save(os.path.join(out_dir, "sec04_comp_fair.png"))
sec04_fair.save(os.path.join(out_dir, "sec04_dynamics.png"))
print("Saved sec04_comp_fair.png:", sec04_fair.size)

# If PTEK + V-MU dump is available, crop dedicated faces:
ptek_vmu_candidates = [
    os.path.join(out_dir, "masteraid_gui_ptek_vmu.png"),
    os.path.join("target/shots_test", "masteraid_gui_ptek_vmu.bmp"),
    os.path.join("target/shots_verify", "masteraid_gui_ptek_vmu.bmp"),
    os.path.join(os.environ.get("MASTERAID_SCREENSHOT_DIR", ""), "masteraid_gui_ptek_vmu.bmp"),
]
ptek_vmu_path = next((p for p in ptek_vmu_candidates if os.path.exists(p)), None)
if ptek_vmu_path:
    img_pv = Image.open(ptek_vmu_path)
    sec03_ptek = img_pv.crop((240, 134, 1120, 502))
    sec03_ptek.save(os.path.join(out_dir, "sec03_eq_ptek.png"))
    print("Saved sec03_eq_ptek.png:", sec03_ptek.size, "from", ptek_vmu_path)

    sec04_vmu = img_pv.crop((240, 496, 1120, 866))
    sec04_vmu.save(os.path.join(out_dir, "sec04_comp_vmu.png"))
    print("Saved sec04_comp_vmu.png:", sec04_vmu.size, "from", ptek_vmu_path)

# 4. Right Wing: Limiter & LUFS
sec08 = img_dark.crop((1116, 134, 1336, 866))
sec08.save(os.path.join(out_dir, "sec08_limiter.png"))
print("Saved sec08_limiter.png:", sec08.size)

# 5. Top Deck: Tape & Clipper
sec06_07 = img_dark.crop((20, 10, 1340, 140))
sec06_07.save(os.path.join(out_dir, "sec06_07_topdeck.png"))
print("Saved sec06_07_topdeck.png:", sec06_07.size)

# 6. Side-by-side Dark and Light theme comparison
half_w = w // 2
theme_comp = Image.new('RGB', (w, h))
theme_comp.paste(img_dark.crop((0, 0, half_w, h)), (0, 0))
theme_comp.paste(img_light.crop((half_w, 0, w, h)), (half_w, 0))
theme_comp.save(os.path.join(out_dir, "sec10_themes.png"))
print("Saved sec10_themes.png:", theme_comp.size)
