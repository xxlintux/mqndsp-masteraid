#!/usr/bin/env python3
"""
Generate masteraid_console.svg
Uses the modular composer architecture (tools/composer.py and tools/components/)
"""

import os
import sys

# Ensure tools directory is in sys.path
sys.path.insert(0, os.path.dirname(__file__))

from composer import compose_console

def generate_svg():
    return compose_console()

if __name__ == "__main__":
    out_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "assets", "visuals", "console"))
    os.makedirs(out_dir, exist_ok=True)
    out_file = os.path.join(out_dir, "masteraid_console.svg")

    svg_content = generate_svg()
    with open(out_file, "w", encoding="utf-8") as f:
        f.write(svg_content)
    
    print(f"Generated {out_file} ({len(svg_content.encode('utf-8'))} bytes)")
