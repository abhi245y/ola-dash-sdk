#!/usr/bin/env python3
"""Rebuilds host/ui/fonts/OlaDashHost-*.ttf from the static Figtree files.
Usage: tools/subset-host-font.py <dir with Figtree-{Medium,Bold,ExtraBold}.ttf> [--evil]
Source: https://github.com/erikdkennedy/figtree/tree/032dfa7fe219ef3a02890d6d3add84eacc9aebfe/fonts/ttf
(SIL OFL 1.1; byte-identical OFL.txt in github.com/google/fonts ofl/figtree). Needs fontTools
(pip install fonttools). The family is renamed because the OFL asks for it and because the host
must own a name no theme font can share (see docs/themes.md). Slint has no OpenType feature
switch, so the digits are remapped to Figtree's tabular figures (its `tnum` lookup).
Keep the weights in step with host/ui (500 default, 700, 800) and add characters to UNICODES
when a host string needs one."""
import re, sys
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont

FAMILY = "OlaDash Host"
UNICODES = [*range(0x20, 0x7F), 0xA0, 0xB0, 0xB7, 0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026]
root = Path(__file__).resolve().parent.parent
out = root / "host/ui/fonts"
args = [a for a in sys.argv[1:] if not a.startswith("--")]
if len(args) != 1:
    sys.exit(__doc__)
src = Path(args[0])
evil = "--evil" in sys.argv
if evil:  # tools/fixtures/refused/evil4: a one-glyph font claiming the host name
    UNICODES = [0x41]
    out = root / "tools/fixtures/refused/evil4/fonts"
out.mkdir(parents=True, exist_ok=True)


def tabular_digits(font):
    gsub = font["GSUB"].table
    tnum = {}
    for fr in gsub.FeatureList.FeatureRecord:
        if fr.FeatureTag == "tnum":
            for i in fr.Feature.LookupListIndex:
                for st in gsub.LookupList.Lookup[i].SubTable:
                    tnum.update(getattr(getattr(st, "ExtSubTable", st), "mapping", {}))
    for table in font["cmap"].tables:
        for cp in range(0x30, 0x3A):
            if table.cmap.get(cp) in tnum:
                table.cmap[cp] = tnum[table.cmap[cp]]


for style in ("Medium",) if evil else ("Medium", "Bold", "ExtraBold"):
    font = TTFont(src / f"Figtree-{style}.ttf")
    tabular_digits(font)
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
            rec.string = re.sub("Figtree", FAMILY if rec.nameID in (1, 4, 16, 21) else FAMILY.replace(" ", ""), str(rec))
    font.save(out / f"OlaDashHost-{style}.ttf")
