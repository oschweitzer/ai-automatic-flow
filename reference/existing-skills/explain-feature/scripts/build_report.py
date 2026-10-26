#!/usr/bin/env python3
"""Builds a self-contained HTML report from an explanation (markdown) and an optional narration script.

Usage:
  build_report.py --md explanation.md --out report.html [--narration narration.txt] [--audio narration.m4a] [--lang fr] [--open]

Paths for --audio and images inside the markdown are relative to the output file.
"""
import argparse
import html
import os
import subprocess
import sys

TEMPLATE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "report-template.html")


def safe_for_script(text: str) -> str:
    # Content lives inside <script type="text/..."> blocks: only "</script" can break out.
    return text.replace("</script", "<\\/script").replace("</SCRIPT", "<\\/SCRIPT")


def open_file(path: str) -> None:
    opener = "open" if sys.platform == "darwin" else ("start" if os.name == "nt" else "xdg-open")
    try:
        if os.name == "nt":
            os.startfile(path)  # type: ignore[attr-defined]
        else:
            subprocess.Popen([opener, path], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except Exception as exc:  # no desktop / no opener: the path is printed anyway
        print(f"(could not open automatically: {exc})")


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--md", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--narration")
    p.add_argument("--audio", default="")
    p.add_argument("--lang", default="fr")
    p.add_argument("--title", default="Explication")
    p.add_argument("--open", action="store_true")
    a = p.parse_args()

    with open(TEMPLATE, encoding="utf-8") as f:
        tpl = f.read()
    with open(a.md, encoding="utf-8") as f:
        md = f.read()
    narration = ""
    if a.narration and os.path.exists(a.narration):
        with open(a.narration, encoding="utf-8") as f:
            narration = f.read()

    out = (
        tpl.replace("{{MARKDOWN}}", safe_for_script(md))
        .replace("{{NARRATION}}", safe_for_script(narration))
        .replace("{{LANG}}", html.escape(a.lang, quote=True))
        .replace("{{TITLE}}", html.escape(a.title))
        .replace("{{AUDIO}}", html.escape(a.audio, quote=True))
    )

    os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write(out)
    print(os.path.abspath(a.out))
    if a.open:
        open_file(os.path.abspath(a.out))


if __name__ == "__main__":
    main()
