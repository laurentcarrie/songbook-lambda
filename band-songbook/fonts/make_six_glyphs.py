#!/usr/bin/env python3
"""Build songbook_six.ttf: the sixth chords (C6) and the minor sixth chords (Cm6), natural, flat and sharp.

Layout (7 letters A..G per row):
  A..G  6            H..N  flat 6          O..U  sharp 6
  V..\\  m6           c..i  flat m6         j..p  sharp m6      (V W X Y Z [ \\)
The letters (with their flat or sharp sign) come from songbook.ttf, songbook_flat.ttf and songbook_sharp.ttf.
A "6" (Comic Neue Bold, OFL licence) goes where the 7 of the 7 chords sits. The minor sixths are the glyphs of
the minor sevenths (letter, dash, accidental) with their 7 replaced by a 6.
The container is a copy of songbook_sus.ttf, reduced to these characters and renamed.
Usage (needs fontTools; /usr/bin/python3 has it): make_six_glyphs.py
"""
import shutil
from fontTools.ttLib import TTFont
from fontTools import subset
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.recordingPen import RecordingPen

COMIC = "/usr/share/fonts/opentype/comic-neue/ComicNeue-Bold.otf"
OUT = "songbook_six.ttf"
LETTERS = "ABCDEFG"
# plain sixths: (source font, first character, box of the 7 of the 7 chords in that font)
PLAIN = [("songbook.ttf", "A", (676, 468, 970, 872)), ("songbook_flat.ttf", "H", (710, 24, 1003, 429)), ("songbook_sharp.ttf", "O", (730, 10, 979, 354))]
# minor sixths: (source font, first character in that font and in this one)
MINOR = [("songbook.ttf", "V", "V"), ("songbook_flat.ttf", "V", "c"), ("songbook_sharp.ttf", "V", "j")]
EMBOLDEN = [(0, 0), (-8, 0), (8, 0), (0, -8), (0, 8)]


def six_pen(pen, comic, box):
    gs = comic.getGlyphSet(); name = comic.getBestCmap()[ord("6")]
    bp = BoundsPen(gs); gs[name].draw(bp); x0, y0, x1, y1 = bp.bounds
    inner = 8
    s = min((box[2] - box[0] - 2 * inner) / (x1 - x0), (box[3] - box[1] - 2 * inner) / (y1 - y0))
    cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
    for dx, dy in EMBOLDEN:
        t = (s, 0, 0, s, cx - s * (x0 + x1) / 2 + dx, cy - s * (y0 + y1) / 2 + dy)
        gs[name].draw(TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=True), t))


def bbox(pts):
    xs = [p[0] for p in pts]; ys = [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def is_seven(pts):
    x0, y0, x1, y1 = bbox(pts)
    return x0 >= 640 and 200 <= x1 - x0 <= 310 and 290 <= y1 - y0 <= 420


def build():
    comic = TTFont(COMIC)
    shutil.copy("songbook_sus.ttf", OUT)
    f = TTFont(OUT); gl = f["glyf"]; hm = f["hmtx"]; cm = f.getBestCmap()
    for src_name, first, box in PLAIN:
        src = TTFont(src_name); src_cm = src.getBestCmap(); sgs = src.getGlyphSet()
        for i, letter in enumerate(LETTERS):
            pen = TTGlyphPen(None); sgs[src_cm[ord(letter)]].draw(pen); six_pen(pen, comic, box)
            g = pen.glyph(); g.recalcBounds(gl); name = cm[ord(chr(ord(first) + i))]
            gl[name] = g; hm[name] = (src["hmtx"][src_cm[ord(letter)]][0], g.xMin)
    for src_name, src_first, first in MINOR:
        src = TTFont(src_name); src_cm = src.getBestCmap(); sgs = src.getGlyphSet()
        for i in range(7):
            ch = chr(ord(src_first) + i)
            rec = RecordingPen(); sgs[src_cm[ord(ch)]].draw(rec)       # the minor seventh: letter, dash, accidental and the 7
            tmp = TTGlyphPen(None); box = None; contour = []
            for op, args in rec.value:
                contour.append((op, args))
                if op in ("closePath", "endPath"):
                    pts = [p for _, a in contour for p in a if p is not None]
                    if is_seven(pts): box = bbox(pts)                  # the 7 is left out
                    else:
                        for o, a in contour: getattr(tmp, o)(*a)
                    contour = []
            assert box is not None, (src_name, ch)
            six_pen(tmp, comic, box)                                   # a 6 in the place of the 7
            g = tmp.glyph(); g.recalcBounds(gl)
            name = cm[ord(chr(ord(first) + i))]
            gl[name] = g; hm[name] = (src["hmtx"][src_cm[ord(ch)]][0], g.xMin)
    f.save(OUT)
    f = TTFont(OUT)
    keep = [ord(c) for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\cdefghijklmnop"] + [0x20]
    opts = subset.Options(); opts.glyph_names = True; opts.legacy_cmap = True; opts.notdef_outline = True; opts.notdef_glyph = True
    opts.name_IDs = ["*"]; opts.name_languages = ["*"]; opts.layout_features = ["*"]; opts.hinting = True; opts.drop_tables = []
    sub = subset.Subsetter(opts); sub.populate(unicodes=keep); sub.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 17):
            rec.string = "songbook_six"
    f.save(OUT)
    print(OUT, "built:", len(f.getGlyphOrder()), "glyphs")


if __name__ == "__main__":
    build()
