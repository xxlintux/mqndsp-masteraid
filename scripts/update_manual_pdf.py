# -*- coding: utf-8 -*-
"""
Compiles assets/manual.html into assets/Masteraid_User_Manual.pdf using headless Microsoft Edge.
"""
import os
import subprocess
import sys

def build_pdf():
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    html_path = os.path.join(root, "assets", "manual.html")
    pdf_path = os.path.join(root, "assets", "Masteraid_User_Manual.pdf")

    if not os.path.exists(html_path):
        print(f"Error: {html_path} does not exist.", file=sys.stderr)
        sys.exit(1)

    edge_candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ]
    edge = next((p for p in edge_candidates if os.path.exists(p)), None)
    if not edge:
        print("Microsoft Edge not found on this system.", file=sys.stderr)
        sys.exit(1)

    html_abs = html_path.replace(os.sep, "/")
    cmd = [
        edge,
        "--headless",
        "--disable-gpu",
        "--no-pdf-header-footer",
        f"--print-to-pdf={pdf_path}",
        f"file:///{html_abs}"
    ]

    subprocess.run(cmd, check=True)
    print("PDF regenerated at:", pdf_path, "Size:", os.path.getsize(pdf_path))

if __name__ == "__main__":
    build_pdf()
