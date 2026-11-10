#!/usr/bin/env python3
"""Writes the LaTeX source of the chord sheet: every chord macro of chords.tex, one row per letter (A..G).

Three tables (natural, flat, sharp), a row per letter and a column per kind of chord (major, minor, 6, 7, ...).
A macro is named \\chord<letter>[f|s][m]<alteration>, with the alterations of src/chords/glyph.rs, so the
names are built from those parts, and the sheet checks that the macros of chords.tex are all found: one it
does not know goes in a last table, so a chord added to chords.tex is never silently left out of the sheet.
A column with no chord at all is not drawn. A cell is empty when its macro does not exist.
Usage: chords_sheet.py src/resources/texfiles/chords.tex sandbox/chords/all-chords.tex
"""
import re
import sys

LETTERS = "ABCDEFG"
ACCIDENTALS = [("", "natural"), ("f", "flat"), ("s", "sharp")]
# the alteration part of a macro name (glyph.rs), in the order of the columns
ALTERATIONS = [("", ""), ("six", "6"), ("sept", "7"), ("septM", "7M"), ("nine", "9"), ("sustwo", "sus2"),
               ("susfour", "sus"), ("septsusfour", "7sus"), ("dim", "dim"), ("nofith", "5")]
KINDS = [("", "major", ""), ("m", "minor", "m")] \
    + [(alt, label, "") for alt, label in ALTERATIONS if alt] \
    + [("m" + alt, "m" + label, "m") for alt, label in ALTERATIONS if alt]


def macros_of(path):
    return set(re.findall(r"^\\newcommand\\chord([A-Za-z0-9]+)", open(path).read(), re.M)) - {"bass"}


def main(chords_tex, out):
    known = macros_of(chords_tex)
    used = set()
    tex = [r"\documentclass[landscape]{article}",
           r"\usepackage[a4paper,margin=1cm]{geometry}",
           r"\usepackage{fontspec}", r"\usepackage{adjustbox}", r"\usepackage{needspace}",
           r"\pagestyle{empty}", r"\input{chords}", r"\begin{document}",
           r"\setlength{\tabcolsep}{5pt}\renewcommand{\arraystretch}{1.5}\setlength{\parindent}{0pt}"]
    for acc, title in ACCIDENTALS:
        names = {(letter, kind): letter + acc + kind for letter in LETTERS for kind, _, _ in KINDS}
        cols = [(kind, label) for kind, label, _ in KINDS if any(names[(l, kind)] in known for l in LETTERS)]
        tex.append(r"\Needspace{8cm}\par\medskip{\large\bfseries %s}\par\smallskip" % title)
        tex.append(r"\begin{adjustbox}{max width=\textwidth}\begin{tabular}{l|%s}" % ("c" * len(cols)))
        tex.append(" & ".join([""] + [r"{\tiny\ttfamily %s}" % (label or "maj") for _, label in cols]) + r" \\ \hline")
        for letter in LETTERS:
            cells = []
            for kind, _ in cols:
                name = names[(letter, kind)]
                if name in known:
                    used.add(name)
                    cells.append(r"\chord%s" % name)
                else:
                    cells.append("")
            tex.append(" & ".join([r"{\bfseries %s}" % letter] + cells) + r" \\")
        tex.append(r"\end{tabular}\end{adjustbox}\par")
    others = sorted(known - used)
    if others:
        tex.append(r"\par\medskip{\large\bfseries others}\par\smallskip")
        tex.append(r"\begin{tabular}{*{%d}{c}}" % len(others))
        tex.append(" & ".join(r"\shortstack{\chord%s\\[2pt]{\tiny\ttfamily %s}}" % (n, n) for n in others))
        tex.append(r"\end{tabular}\par")
    tex.append(r"\end{document}")
    open(out, "w").write("\n".join(tex) + "\n")
    print("%d chords, %d in the tables, %d others" % (len(known), len(used), len(others)), file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
