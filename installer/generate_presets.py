# -*- coding: utf-8 -*-
"""
Generate Masteraid Factory Presets for the Installer
"""

import os

os.makedirs("installer/presets", exist_ok=True)

presets = [
    {
        "name": "DEFAULT MASTER",
        "tape_speed_30ips": 1,
        "tape_formula_gp9": 0,
        "tape_drive": 0.0,
        "tape_bias": 0.0,
        "tape_bump": 0.0,
        "tape_bump_freq": 60.0,
        "tape_flutter": 0.0,
        "clipper_drive": 0.0,
        "clipper_ceiling": -0.1,
        "clipper_hard_shape": 0,
        "clipper_os_4x": 1,
        "eq_model": "Ptek",
        "comp_model": "Fair",
        "comp_thresh": 0.0,
        "comp_ratio": 2.0,
        "comp_attack": 10.0,
        "comp_release": 100.0,
        "comp_mix": 100.0,
        "limiter_gain": 0.0,
        "limiter_ceiling": -0.1,
        "limiter_lookahead": 1.0,
        "limiter_release": 0.0,
        "limiter_lift": 0.0,
        "limiter_true_peak": 1,
        "limiter_punch": 0,
    },
    {
        "name": "ANALOG TAPE GLUE",
        "tape_speed_30ips": 1,
        "tape_formula_gp9": 0,
        "tape_drive": 2.5,
        "tape_bias": -1.5,
        "tape_bump": 3.0,
        "tape_bump_freq": 70.0,
        "tape_flutter": 12.0,
        "clipper_drive": 1.0,
        "clipper_ceiling": -0.2,
        "clipper_hard_shape": 0,
        "clipper_os_4x": 1,
        "eq_model": "Ptek",
        "comp_model": "Fair",
        "comp_thresh": -8.0,
        "comp_ratio": 2.5,
        "comp_attack": 0.4,
        "comp_release": 800.0,
        "comp_mix": 100.0,
        "limiter_gain": 1.5,
        "limiter_ceiling": -0.2,
        "limiter_lookahead": 1.2,
        "limiter_release": 0.0,
        "limiter_lift": 0.0,
        "limiter_true_peak": 1,
        "limiter_punch": 1,
    },
    {
        "name": "STREAMING MASTER (-14 LUFS)",
        "tape_speed_30ips": 1,
        "tape_formula_gp9": 1,
        "tape_drive": 1.0,
        "tape_bias": 0.0,
        "tape_bump": 1.5,
        "tape_bump_freq": 60.0,
        "tape_flutter": 0.0,
        "clipper_drive": 1.5,
        "clipper_ceiling": -0.3,
        "clipper_hard_shape": 0,
        "clipper_os_4x": 1,
        "eq_model": "Mass",
        "comp_model": "VMu",
        "comp_thresh": -6.0,
        "comp_ratio": 2.0,
        "comp_attack": 15.0,
        "comp_release": 150.0,
        "comp_mix": 100.0,
        "limiter_gain": 2.0,
        "limiter_ceiling": -1.0,
        "limiter_lookahead": 1.0,
        "limiter_release": 0.0,
        "limiter_lift": 5.0,
        "limiter_true_peak": 1,
        "limiter_punch": 1,
    },
    {
        "name": "ACOUSTIC AIR & TRANSPARENCY",
        "tape_speed_30ips": 1,
        "tape_formula_gp9": 1,
        "tape_drive": 0.5,
        "tape_bias": 1.0,
        "tape_bump": 0.5,
        "tape_bump_freq": 50.0,
        "tape_flutter": 5.0,
        "clipper_drive": 0.0,
        "clipper_ceiling": -0.1,
        "clipper_hard_shape": 0,
        "clipper_os_4x": 1,
        "eq_model": "Ptek",
        "comp_model": "VMu",
        "comp_thresh": -4.0,
        "comp_ratio": 1.8,
        "comp_attack": 25.0,
        "comp_release": 250.0,
        "comp_mix": 85.0,
        "limiter_gain": 1.0,
        "limiter_ceiling": -0.3,
        "limiter_lookahead": 1.5,
        "limiter_release": 0.0,
        "limiter_lift": 10.0,
        "limiter_true_peak": 1,
        "limiter_punch": 0,
    },
    {
        "name": "MODERN CLUB & BASS PUNCH",
        "tape_speed_30ips": 0,
        "tape_formula_gp9": 0,
        "tape_drive": 3.0,
        "tape_bias": -2.0,
        "tape_bump": 4.5,
        "tape_bump_freq": 80.0,
        "tape_flutter": 8.0,
        "clipper_drive": 3.5,
        "clipper_ceiling": -0.2,
        "clipper_hard_shape": 1,
        "clipper_os_4x": 1,
        "eq_model": "Mass",
        "comp_model": "Fair",
        "comp_thresh": -12.0,
        "comp_ratio": 4.0,
        "comp_attack": 0.2,
        "comp_release": 300.0,
        "comp_mix": 90.0,
        "limiter_gain": 4.0,
        "limiter_ceiling": -0.1,
        "limiter_lookahead": 0.8,
        "limiter_release": 0.0,
        "limiter_lift": 0.0,
        "limiter_true_peak": 1,
        "limiter_punch": 1,
    },
]

for p in presets:
    text = (
        f"PRESET:{p['name']}\n"
        f"tape_speed_30ips={p['tape_speed_30ips']}\n"
        f"tape_formula_gp9={p['tape_formula_gp9']}\n"
        f"tape_drive={p['tape_drive']:.3f}\n"
        f"tape_bias={p['tape_bias']:.3f}\n"
        f"tape_bump={p['tape_bump']:.3f}\n"
        f"tape_bump_freq={p['tape_bump_freq']:.1f}\n"
        f"tape_flutter={p['tape_flutter']:.3f}\n"
        f"clipper_drive={p['clipper_drive']:.3f}\n"
        f"clipper_ceiling={p['clipper_ceiling']:.3f}\n"
        f"clipper_hard_shape={p['clipper_hard_shape']}\n"
        f"clipper_os_4x={p['clipper_os_4x']}\n"
        f"eq_model={p['eq_model']}\n"
        f"comp_model={p['comp_model']}\n"
        f"comp_thresh={p['comp_thresh']:.3f}\n"
        f"comp_ratio={p['comp_ratio']:.3f}\n"
        f"comp_attack={p['comp_attack']:.3f}\n"
        f"comp_release={p['comp_release']:.3f}\n"
        f"comp_mix={p['comp_mix']:.3f}\n"
        f"limiter_gain={p['limiter_gain']:.3f}\n"
        f"limiter_ceiling={p['limiter_ceiling']:.3f}\n"
        f"limiter_lookahead={p['limiter_lookahead']:.3f}\n"
        f"limiter_release={p['limiter_release']:.3f}\n"
        f"limiter_lift={p['limiter_lift']:.3f}\n"
        f"limiter_true_peak={p['limiter_true_peak']}\n"
        f"limiter_punch={p['limiter_punch']}\n"
        f"END_PRESET\n"
    )
    fname = f"installer/presets/{p['name']}.mqnpreset"
    with open(fname, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"Wrote {fname}")
