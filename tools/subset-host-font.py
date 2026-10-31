#!/usr/bin/env python3
"""Rebuilds host/ui/fonts/OlaDashHost-*.ttf from the Inter files in themes/default/fonts.
Needs fontTools (pip install fonttools). The family is renamed because the OFL asks for it
and because the host must own a name no theme font can share (see docs/themes.md).
Keep the weights in step with host/ui (400 default, 600, 700) and add characters to UNICODES
when a host string needs one."""
import re, sys
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont

FAMILY = "OlaDash Host"
UNICODES = [*range(0x20, 0x7F), 0xA0, 0xB0, 0xB7, 0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026]
root = Path(__file__).resolve().parent.parent
out = root / "host/ui/fonts"
evil = "--evil" in sys.argv
if evil:  # tools/fixtures/refused/evil4: a one-glyph font claiming the host name
    UNICODES = [0x41]
    out = root / "tools/fixtures/refused/evil4/fonts"
out.mkdir(parents=True, exist_ok=True)
for style in ("Regular",) if evil else ("Regular", "SemiBold", "Bold"):
    font = TTFont(root / f"themes/default/fonts/Inter-{style}.ttf")
    opts = subset.Options()
    if evil:
        opts.layout_features = []
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    sub = subset.Subsetter(opts)
    sub.populate(unicodes=UNICODES)
    sub.subset(font)
    for rec in font["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 21):
            rec.string = re.sub("Inter", FAMILY if rec.nameID in (1, 4, 16, 21) else FAMILY.replace(" ", ""), str(rec))
    font.save(out / f"OlaDashHost-{style}.ttf")
