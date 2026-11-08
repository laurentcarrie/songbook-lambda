#!/usr/bin/env python3
"""Draw the sus2 / sus4 chords in songbook_sus.ttf (and its songbookmac copy).

Layout, like the other songbook fonts (7 letters A..G per row):
  A..G  sus4          H..N  sus2
  O..U  flat sus4     V..\\ flat sus2        (V W X Y Z [ \\)
  c..i  sharp sus4    j..p  sharp sus2
The letter (and its flat or sharp sign) is taken from songbook.ttf, songbook_flat.ttf and
songbook_sharp.ttf; "sus" over the number (Comic Neue Bold, OFL licence) is added at the top right
of a natural letter, or at the lower right when a flat or sharp sign takes the top right.
Usage (needs fontTools; /usr/bin/python3 has it): make_sus_glyphs.py
"""
from fontTools.ttLib import TTFont
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen

COMIC = "/usr/share/fonts/opentype/comic-neue/ComicNeue-Bold.otf"
LETTERS = "ABCDEFG"
# first character of each row of 7 in songbook_sus.ttf: (letter font, accidental, digit)
ROWS = [("songbook.ttf", "A", "4"), ("songbook.ttf", "H", "2"),
        ("songbook_flat.ttf", "O", "4"), ("songbook_flat.ttf", "V", "2"),
        ("songbook_sharp.ttf", "c", "4"), ("songbook_sharp.ttf", "j", "2")]
# annotation geometry in font units (1024 per em): x, baseline of "sus", size of "sus", baseline and size of the digit
TOP_RIGHT = dict(x=620, sus_y=790, sus_size=250, num_y=520, num_size=400)
LOWER_RIGHT = dict(x=650, sus_y=250, sus_size=215, num_y=-40, num_size=330)
EMBOLDEN = [(0, 0), (-7, 0), (7, 0), (0, -7), (0, 7)]   # the thin strokes of the text are thickened a little


def draw_text(pen, comic, text, size, x, y):
    gs = comic.getGlyphSet(); cmap = comic.getBestCmap(); scale = size / comic["head"].unitsPerEm
    cx = x
    for ch in text:
        name = cmap[ord(ch)]
        for dx, dy in EMBOLDEN:
            tp = TransformPen(Cu2QuPen(pen, 1.0, reverse_direction=True), (scale, 0, 0, scale, cx + dx, y + dy))
            gs[name].draw(tp)
        cx += gs[name].width * scale


def build(target_files):
    comic = TTFont(COMIC)
    sources = {n: TTFont(n) for n in {r[0] for r in ROWS}}
    for path in target_files:
        f = TTFont(path); gl = f["glyf"]; hm = f["hmtx"]; cm = f.getBestCmap()
        for src_name, first, digit in ROWS:
            src = sources[src_name]; src_cm = src.getBestCmap(); sgs = src.getGlyphSet()
            geom = TOP_RIGHT if src_name == "songbook.ttf" else LOWER_RIGHT
            for i, letter in enumerate(LETTERS):
                pen = TTGlyphPen(None)
                sgs[src_cm[ord(letter)]].draw(pen)                        # the letter, with its flat or sharp sign
                draw_text(pen, comic, "sus", geom["sus_size"], geom["x"], geom["sus_y"])
                draw_text(pen, comic, digit, geom["num_size"], geom["x"] + 40, geom["num_y"])
                g = pen.glyph(); g.recalcBounds(gl)
                target = chr(ord(first) + i)
                name = cm[ord(target)]
                gl[name] = g
                hm[name] = (src["hmtx"][src_cm[ord(letter)]][0], g.xMin)
        f.save(path)
        print(path, "done")


if __name__ == "__main__":
    build(["songbook_sus.ttf", "songbookmac_sus.ttf"])
