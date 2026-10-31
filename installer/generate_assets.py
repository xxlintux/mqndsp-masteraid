# -*- coding: utf-8 -*-
"""
Generate Windows Setup Wizard Graphics for Inno Setup
"""

from PIL import Image, ImageDraw, ImageFont, ImageFilter
import os

os.makedirs("installer/assets", exist_ok=True)

# 1. Load MQN logo
logo_src = Image.open("source/gui/mqn_logo.png").convert("RGBA")
# Tint logo (#FF007F)
r, g, b, a = logo_src.split()
pink_solid = Image.new("RGBA", logo_src.size, (255, 0, 127, 255))
pink_logo = Image.composite(pink_solid, Image.new("RGBA", logo_src.size, (0, 0, 0, 0)), a)

# -------------------------------------------------------------------------
# A. Wizard Sidebar: 328 x 628 (2x DPI for Inno Setup 6 WizardImageFile)
# -------------------------------------------------------------------------
side_w, side_h = 328, 628
side_img = Image.new("RGB", (side_w, side_h), (11, 13, 18))
draw = ImageDraw.Draw(side_img)

# Subtle background gradient
for y in range(side_h):
    factor = y / side_h
    cur_r = int(11 + 13 * factor)
    cur_g = int(13 + 15 * factor)
    cur_b = int(18 + 20 * factor)
    draw.line([(0, y), (side_w, y)], fill=(cur_r, cur_g, cur_b))

# Left vertical glowing accent bar
for i in range(12):
    alpha_fact = (12 - i) / 12.0
    col = (int(255 * alpha_fact), 0, int(127 * alpha_fact))
    draw.line([(i, 0), (i, side_h)], fill=col)

# Subtle grid lines in background
for gy in range(0, side_h, 32):
    draw.line([(0, gy), (side_w, gy)], fill=(20, 24, 32))
for gx in range(0, side_w, 32):
    draw.line([(gx, 0), (gx, side_h)], fill=(20, 24, 32))

# Subtle glow behind logo
glow_size = (360, 180)
glow = Image.new("RGBA", glow_size, (0, 0, 0, 0))
g_draw = ImageDraw.Draw(glow)
g_draw.ellipse([20, 20, 340, 160], fill=(255, 0, 127, 65))
glow = glow.filter(ImageFilter.GaussianBlur(32))

side_img.paste(glow, (-16, 120), glow)

# Scale and paste logo
target_logo_w = 260
target_logo_h = int(target_logo_w / (882.0 / 257.0))
logo_resized = pink_logo.resize((target_logo_w, target_logo_h), Image.Resampling.LANCZOS)
logo_x = (side_w - target_logo_w) // 2 + 6
logo_y = 175
side_img.paste(logo_resized, (logo_x, logo_y), logo_resized)

# Typography below logo
try:
    font_large = ImageFont.truetype("arialbd.ttf", 26)
    font_sub = ImageFont.truetype("arial.ttf", 13)
    font_foot = ImageFont.truetype("arialbd.ttf", 11)
except Exception:
    font_large = ImageFont.load_default()
    font_sub = ImageFont.load_default()
    font_foot = ImageFont.load_default()

# MASTERAID text
title_text = "MASTERAID"
bbox = draw.textbbox((0, 0), title_text, font=font_large)
tw = bbox[2] - bbox[0]
draw.text(((side_w - tw) // 2 + 4, logo_y + target_logo_h + 20), title_text, fill=(255, 255, 255), font=font_large)

# Subtitle
sub_text = "PRECISION MASTERING CONSOLE"
bbox = draw.textbbox((0, 0), sub_text, font=font_sub)
tw = bbox[2] - bbox[0]
draw.text(((side_w - tw) // 2 + 4, logo_y + target_logo_h + 54), sub_text, fill=(154, 160, 178), font=font_sub)

# Horizontal separator with accent center dot
draw.line([(34, 340), (side_w - 24, 340)], fill=(40, 46, 60))
draw.ellipse([side_w // 2 + 1, 338, side_w // 2 + 7, 342], fill=(255, 0, 127))

# Feature bullets on sidebar
bullets = [
    "64-Bit Double-Precision DSP",
    "ATR-102 Magnetic Tape Saturation",
    "Precision 4x Oversampled Clipper",
    "PTEK Tube & MASS Parametric EQs",
    "FAIR 670 & Modern V-MU Dynamics",
    "Lookahead True Peak Limiter",
    "EBU R128 Broadcast Loudness",
    "CLAP & VST3 Audio Architectures",
]

by = 365
for b in bullets:
    draw.text((36, by), "\u2022", fill=(255, 0, 127), font=font_foot)
    draw.text((50, by), b, fill=(210, 214, 226), font=font_sub)
    by += 28

# Bottom copyright watermark
draw.text((36, side_h - 32), "MQN DSP \u2022 ALL RIGHTS RESERVED", fill=(100, 106, 122), font=font_foot)

side_img.save("installer/assets/wizard_side.bmp", "BMP")
print("Saved installer/assets/wizard_side.bmp")


# -------------------------------------------------------------------------
# B. Wizard Header Small: 110 x 116 (2x DPI for Inno Setup 6 WizardSmallImageFile)
# -------------------------------------------------------------------------
small_w, small_h = 110, 116
small_img = Image.new("RGB", (small_w, small_h), (255, 255, 255))
s_draw = ImageDraw.Draw(small_img)

# Dark clean rectangle
s_draw.rectangle([0, 0, small_w, small_h], fill=(16, 19, 26))
# Bottom accent line
s_draw.line([(0, small_h - 2), (small_w, small_h - 2)], fill=(255, 0, 127), width=2)

# Small logo
sm_w = 92
sm_h = int(sm_w / (882.0 / 257.0))
sm_resized = pink_logo.resize((sm_w, sm_h), Image.Resampling.LANCZOS)
sm_x = (small_w - sm_w) // 2
sm_y = (small_h - sm_h) // 2
small_img.paste(sm_resized, (sm_x, sm_y), sm_resized)

small_img.save("installer/assets/wizard_small.bmp", "BMP")
print("Saved installer/assets/wizard_small.bmp")


# -------------------------------------------------------------------------
# C. Application Icon: Multi-Resolution .ICO (256, 128, 64, 48, 32, 16)
# -------------------------------------------------------------------------
ico_sizes = [256, 128, 64, 48, 32, 16]
ico_images = []

for sz in ico_sizes:
    canvas = Image.new("RGBA", (sz, sz), (0, 0, 0, 0))
    c_draw = ImageDraw.Draw(canvas)
    
    pad = max(1, sz // 16)
    r = sz // 6
    c_draw.rounded_rectangle([pad, pad, sz - pad, sz - pad], radius=r, fill=(16, 19, 26, 255), outline=(255, 0, 127, 255), width=max(1, sz // 32))
    
    lg_w = sz - pad * 6
    lg_h = int(lg_w / (882.0 / 257.0))
    if lg_w > 0 and lg_h > 0:
        lg_res = pink_logo.resize((lg_w, lg_h), Image.Resampling.LANCZOS)
        canvas.paste(lg_res, ((sz - lg_w) // 2, (sz - lg_h) // 2), lg_res)
    
    ico_images.append(canvas)

ico_images[0].save(
    "installer/assets/app_icon.ico",
    format="ICO",
    sizes=[(sz, sz) for sz in ico_sizes],
    append_images=ico_images[1:]
)
print("Saved installer/assets/app_icon.ico")
