#!/usr/bin/env python3
"""Build songbook_sevensus.ttf: the 7sus4 chords (G7sus4), natural, flat and sharp.

Layout (7 letters A..G per row):
  A..G  7sus4        H..N  flat 7sus4        O..U  sharp 7sus4
The letters (with their flat or sharp sign) come from songbook.ttf, songbook_flat.ttf and songbook_sharp.ttf.
A "7" (Comic Neue Bold, OFL licence) goes where the 7 of the 7 chords sits, and "sus4" in smaller letters
follows it, on its baseline. The glyph is wider than the other chords: its advance covers the "sus4".
The container is a copy of songbook_sus.ttf, reduced to these characters and renamed.
Usage (needs fontTools; /usr/bin/python3 has it): make_sevensus_glyphs.py
"""
import shutil
from fontTools.ttLib import TTFont
from fontTools import subset
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

COMIC = "/usr/share/fonts/opentype/comic-neue/ComicNeue-Bold.otf"
OUT = "songbook_sevensus.ttf"
LETTERS = "ABCDEFG"
# (source font, first character in this font, box of the 7 of the 7 chords in that font)
ROWS = [("songbook.ttf", "A", (676, 468, 970, 872)),
        ("songbook_flat.ttf", "H", (710, 24, 1003, 429)),
        ("songbook_sharp.ttf", "O", (730, 10, 979, 354))]
SUS_SIZE = 500     # size of "sus4", in font units per em
GAP = 30           # between the 7 and "sus4"
EMBOLDEN = [(0, 0), (-8, 0), (8, 0), (0, -8), (0, 8)]
EMBOLDEN_SUS = [(0, 0), (-3, 0), (3, 0)]   # "sus4" is small: a lighter stroke keeps the letters open


def draw_char(pen, comic, ch, scale, x, y, bold=EMBOLDEN):
    gs = comic.getGlyphSet(); name = comic.getBestCmap()[ord(ch)]
    for dx, dy in bold:
        gs[name].draw(TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=True), (scale, 0, 0, scale, x + dx, y + dy)))
    return gs[name].width * scale


def seven_and_sus4(pen, comic, box):
    """Draws the 7 in `box`, then "sus4" after it. Returns the x where everything ends."""
    gs = comic.getGlyphSet(); cmap = comic.getBestCmap()
    bp = BoundsPen(gs); gs[cmap[ord("7")]].draw(bp); x0, y0, x1, y1 = bp.bounds
    inner = 8
    s = min((box[2] - box[0] - 2 * inner) / (x1 - x0), (box[3] - box[1] - 2 * inner) / (y1 - y0))
    cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
    x_seven = cx - s * (x0 + x1) / 2
    y_seven = cy - s * (y0 + y1) / 2
    draw_char(pen, comic, "7", s, x_seven, y_seven)
    scale = SUS_SIZE / comic["head"].unitsPerEm
    x = box[2] + GAP
    for ch in "sus4":
        x += draw_char(pen, comic, ch, scale, x, box[1] + inner, EMBOLDEN_SUS)
    return x


def build():
    comic = TTFont(COMIC)
    shutil.copy("songbook_sus.ttf", OUT)
    f = TTFont(OUT); gl = f["glyf"]; hm = f["hmtx"]; cm = f.getBestCmap()
    for src_name, first, box in ROWS:
        src = TTFont(src_name); src_cm = src.getBestCmap(); sgs = src.getGlyphSet()
        for i, letter in enumerate(LETTERS):
            pen = TTGlyphPen(None); sgs[src_cm[ord(letter)]].draw(pen)
            end = seven_and_sus4(pen, comic, box)
            g = pen.glyph(); g.recalcBounds(gl); name = cm[ord(chr(ord(first) + i))]
            gl[name] = g; hm[name] = (max(src["hmtx"][src_cm[ord(letter)]][0], int(end) + 40), g.xMin)
    f.save(OUT)
    f = TTFont(OUT)
    keep = [ord(c) for c in "ABCDEFGHIJKLMNOPQRSTU"] + [0x20]
    opts = subset.Options(); opts.glyph_names = True; opts.legacy_cmap = True; opts.notdef_outline = True; opts.notdef_glyph = True
    opts.name_IDs = ["*"]; opts.name_languages = ["*"]; opts.layout_features = ["*"]; opts.hinting = True; opts.drop_tables = []
    sub = subset.Subsetter(opts); sub.populate(unicodes=keep); sub.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 17):
            rec.string = "songbook_sevensus"
    f.save(OUT)
    print(OUT, "built:", len(f.getGlyphOrder()), "glyphs")


if __name__ == "__main__":
    build()
