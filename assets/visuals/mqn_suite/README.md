# MQN Tactile Suite
**The Official Masteraid / Mastering Chain Tactile Control Design System**

MQN Suite is the definitive hardware-grade skeuomorphic control library for Masteraid applications and console interfaces. Every control is engineered to exact physical tolerances with mathematically locked proportions, zero raster blur, embedded typography, and pure neon magenta accenting.

---

## 💎 Design Tokens & Physical Invariants
* **Accent Token**: `#FF007F` (Pure Neon Magenta). Universal active tick, LED glow, and cursor highlight.
* **Knob Enclosure Geometry**: Single knob rotor ($R = 46.0\text{px}$) and concentric outer knurled collar ($R = 46.0\text{px}$) share identical $92.0\text{px}$ outer diameters.
* **Concentric Inner Rotor**: $R = 40.0\text{px}$ ($80.0\text{px}$ diameter) machined aluminum face with inset tactile cap.
* **Outer Collar Pointer**: Starts at $R = 40.0\text{px}$ and terminates strictly at $R = 48.0\text{px}$ ($2.0\text{px}$ protrusion past the knurled rim).
* **Dial Ticks Radial Geometry**: Radial bounds $R_1 = 46.5\text{px}$ to $R_2 = 54.0\text{px}$ ($7.5\text{px}$ uniform length, stroke-width $1.4\text{px}$). Active tick switches to `#FF007F` without modifying stroke size.
* **Segmented Starburst Readout**: Authentic $84 \times 32\text{px}$ beveled well with inner shadow (`filter7_i_82_544`). Embeds 6-character 14-segment `DSEG14 Classic Italic` directly as base64 WOFF2 for 100% offline self-containment.

---

## 📁 Directory Structure

```
assets/mqn_suite/
├── manifest.json                  # Machine-readable token & asset catalog
├── README.md                      # Design system specification
├── fonts/
│   └── dseg14-classic-italic.woff2
├── dark/
│   ├── assembled/                 # 160x200 ready-to-mount cards
│   │   ├── knob_single_readout_top.svg
│   │   ├── knob_single_readout_bottom.svg
│   │   ├── knob_concentric_readout_top.svg
│   │   └── knob_concentric_readout_bottom.svg
│   └── components/                # Standalone building blocks
│       ├── plate_well_160x200.svg
│       ├── readout_segmented_6cell.svg
│       ├── dial_ticks_single.svg
│       ├── dial_ticks_concentric_upper.svg
│       ├── dial_ticks_concentric_lower.svg
│       ├── dial_ticks_concentric_complete.svg
│       ├── rotor_single_knob.svg
│       ├── rotor_concentric_outer_ring.svg
│       └── rotor_concentric_inner_main.svg
└── light/
    ├── assembled/                 # 160x200 ready-to-mount cards (Light theme)
    │   ├── knob_single_readout_top.svg
    │   ├── knob_single_readout_bottom.svg
    │   ├── knob_concentric_readout_top.svg
    │   └── knob_concentric_readout_bottom.svg
    └── components/                # Standalone building blocks (Light theme)
        ├── plate_well_160x200.svg
        ├── readout_segmented_6cell.svg
        ├── dial_ticks_single.svg
        ├── dial_ticks_concentric_upper.svg
        ├── dial_ticks_concentric_lower.svg
        ├── dial_ticks_concentric_complete.svg
        ├── rotor_single_knob.svg
        ├── rotor_concentric_outer_ring.svg
        └── rotor_concentric_inner_main.svg
```

---

## 🛠 Usage in Plugins & GUIs
All SVGs can be rendered directly in browser DOMs, JUCE `juce::Drawable`, or React Native SVG components.

To rotate the inner knob programmatically in DOM/SVG:
```javascript
document.querySelector('#inner-knob-rotor').setAttribute('transform', `rotate(${angle}, 80, 126)`);
```

To update the 6-character 14-segment readout:
```javascript
document.querySelector('#active-readout-text').textContent = "+3.5dB".padStart(6, ' ');
```
