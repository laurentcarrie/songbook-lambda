#!/usr/bin/env python3
"""Build songbook_five.ttf: the power chords (C5), natural, flat and sharp.

Layout (7 letters A..G per row): A..G natural 5, H..N flat 5, O..U sharp 5.
Each glyph is the letter (with its flat or sharp sign) of songbook.ttf, songbook_flat.ttf or
songbook_sharp.ttf, plus a "5" (Comic Neue Bold, OFL licence) where the 7 of the 7 chords sits:
at the top right of a natural letter, at the lower right under a flat or sharp sign.
The container is a copy of songbook_sus.ttf, reduced to these 21 characters and renamed.
Usage (needs fontTools; /usr/bin/python3 has it): make_five_glyphs.py
"""
import shutil
from fontTools.ttLib import TTFont
from fontTools import subset
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

COMIC = "/usr/share/fonts/opentype/comic-neue/ComicNeue-Bold.otf"
OUT = "songbook_five.ttf"
LETTERS = "ABCDEFG"
# (source font, first character, bounding box of the 7 in that font: the "5" is fitted to the same box)
ROWS = [("songbook.ttf", "A", (676, 468, 970, 872)),
        ("songbook_flat.ttf", "H", (710, 24, 1003, 429)),
        ("songbook_sharp.ttf", "O", (730, 10, 979, 354))]
EMBOLDEN = [(0, 0), (-8, 0), (8, 0), (0, -8), (0, 8)]


def five_pen(pen, comic, box):
    gs = comic.getGlyphSet(); name = comic.getBestCmap()[ord("5")]
    bp = BoundsPen(gs); gs[name].draw(bp); x0, y0, x1, y1 = bp.bounds
    inner = 8                                          # keep the emboldened outline inside the box
    s = min((box[2] - box[0] - 2 * inner) / (x1 - x0), (box[3] - box[1] - 2 * inner) / (y1 - y0))
    cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
    for dx, dy in EMBOLDEN:
        t = (s, 0, 0, s, cx - s * (x0 + x1) / 2 + dx, cy - s * (y0 + y1) / 2 + dy)
        gs[name].draw(TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=True), t))


def build():
    comic = TTFont(COMIC)
    shutil.copy("songbook_sus.ttf", OUT)
    f = TTFont(OUT); gl = f["glyf"]; hm = f["hmtx"]; cm = f.getBestCmap()
    for src_name, first, box in ROWS:
        src = TTFont(src_name); src_cm = src.getBestCmap(); sgs = src.getGlyphSet()
        for i, letter in enumerate(LETTERS):
            pen = TTGlyphPen(None)
            sgs[src_cm[ord(letter)]].draw(pen)
            five_pen(pen, comic, box)
            g = pen.glyph(); g.recalcBounds(gl)
            name = cm[ord(chr(ord(first) + i))]
            gl[name] = g
            hm[name] = (src["hmtx"][src_cm[ord(letter)]][0], g.xMin)
    f.save(OUT)
    # keep only the 21 characters, then rename the font
    f = TTFont(OUT)
    opts = subset.Options(); opts.glyph_names = True; opts.legacy_cmap = True; opts.notdef_outline = True; opts.notdef_glyph = True
    opts.name_IDs = ["*"]; opts.name_languages = ["*"]; opts.layout_features = ["*"]; opts.hinting = True; opts.drop_tables = []
    sub = subset.Subsetter(opts); sub.populate(unicodes=[ord(c) for c in "ABCDEFGHIJKLMNOPQRSTU"] + [0x20]); sub.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 17):
            rec.string = "songbook_five"
    f.save(OUT)
    print(OUT, "built:", len(f.getGlyphOrder()), "glyphs")


if __name__ == "__main__":
    build()
