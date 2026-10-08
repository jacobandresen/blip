#!/usr/bin/env python3
"""Subsets the two handwriting fonts the manual's note uses to web/*.woff2.

    python3 tools/make_hand_fonts.py Caveat[wght].ttf MrsSaintDelafield-Regular.ttf

Needs fonttools and brotli. Both fonts are under the SIL Open Font License
(web/fonts-OFL.txt): Caveat by Impallari Type, Mrs Saint Delafield by Sudtipos,
both from github.com/google/fonts. Only the characters the note and the
signature draw are kept.
"""

import sys
from pathlib import Path
from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

WEB = Path(__file__).resolve().parent.parent / "web"
LETTERS = "".join(chr(c) for c in range(0x20, 0x7F))


def subset(font, text, out):
    options = Options()
    options.flavor = "woff2"
    options.layout_features = ["*"]
    subsetter = Subsetter(options)
    subsetter.populate(text=text)
    subsetter.subset(font)
    font.flavor = "woff2"
    font.save(out)
    print(f"wrote {out} ({out.stat().st_size:,} bytes)")


caveat, signature = sys.argv[1:3]
subset(instantiateVariableFont(TTFont(caveat), {"wght": 500}), LETTERS, WEB / "caveat.woff2")
subset(TTFont(signature), "Jacob Andresen", WEB / "mrs-saint-delafield.woff2")
