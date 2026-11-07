# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `pdfcellpositions`: each bar has a `lyric` field (the `\songwordfb` syllable of its first beat, `[pickup] syllable`) and a `syllables:` list gives the position of each syllable on the lyrics page. The k-th `\songwordfb` of a section is its k-th bar.
- `world.yml`: a song has `warnings:` when the lyrics of a section have `\songwordfb` tags that do not match its number of bars.

### Added
- `pdfcellpositions`: each bar has a `chords` field (space separated, e.g. `G A`, `Fxm7`, `B/Dx`; `x` sharp, `b` flat, `r` rest), also for the passes of a repeat and for Ref sections, so the apk can play them.

## [0.0.47]

### Added
- `pdfcellpositions`: a `dots:` list gives the position of each dot of a repeat marker (`first`/`last` bars of the pass it stands for, `page`, `x`, `y`, `r` in PDF points), so the apk can mark the pass being played.

## [0.0.46]

### Added

- `delivery/pdfcellpositions/<song>.yml`: the page position of every bar of the
  chord chart in `delivery/pdf/<song>.pdf`, so a viewer can locate bar `n`.
  `page_width_pt` and `page_height_pt` give the page size; each entry of `cells`
  has `bar` (the number printed on the chart, one entry per played bar, repeats
  included), `page`, `section` (the title of its section), and the cell rectangle
  `x`, `y`, `w`, `h`, in PDF points
  (1/72 inch) from the top-left corner of the page. A bar of a reference
  section (`refrain 3`) is the whole box of the reference.

### Changed

- The chord chart is scaled to the text width by the build instead of
  `adjustbox`; the layout is unchanged.

## [0.0.45]

### Added

- `world.yml`: each song carries its `tempo` (beats per minute, from `song.yml`).

## [0.0.44]

### Added

- `world.yml`: each song lists its delivered mp3 files (`mp3`: `name` and `path`),
  the recording (`mp3/<id>.mp3`) and the renders (`mp3-renders/<id>-<name>.mp3`).

## [0.0.43]

### Added

- `world.yml`: an index of the delivery, written at the root of the delivery
  next to `pdf/`. It lists the tags, the songs (id, author, title, tags, pdf)
  and the books (name, tags, songs, pdf). Only written when the build succeeds.
- Raw songs: a `song.yml` can name a ready-made pdf with `raw:`; it is
  delivered as is to the `raw/` folder.

### Changed

- `structure` is optional in `song.yml`: a song without a structure gets no
  generated pdf (it is not usable in a book).

## [0.0.41]

### Added

- Slash chords: `B/D#`, `Bs/Ds`, `Em/B`, `C/Gf` in a chord row. The bass note is
  drawn small under the chord (new `\chordbass` macro in `chords.tex`).

### Changed

- Repeat marks (`x2`, `x3`...) are now a number in a circle, without the `x`.

## [0.0.40]

### Fixed

- Articulations (staccato, accent...) now print on tab staves: `songbook.ily`
  reverts `Script.stencil` on every `TabStaff`, so `c4-.` shows its dot even
  with the rhythm-only tab setups.

### Removed

- `\mypull`, `\mypulled` and `\myrelease`: the guitar articulation macros
  were no longer used by any song.

## [0.0.39]

### Added

- A song can carry an `external_id` in its `info` section, the id of the song
  on an external music service: `external_id: !Deezer "3135556"` or
  `external_id: !Youtube "dQw4w9WgXcQ"`. The field is optional.

## [0.0.38]

### Added

- Every song has an `add.tikz` beside `song.yml`: TikZ drawings mounted into
  the sandbox and inserted at the end of the chord chart. The file is required.
- The chord chart defines a coordinate `BAR_<n>` at the center of every bar;
  `\BAR{n}` names it, e.g. `\path (\BAR{4}) ...`. Repeated rows reuse the
  cells of their first pass, and a Ref section's bars sit at the center of its
  box.
- The chord chart defines a coordinate `SECTION_<id>` for every section, on
  the right edge of its bar grid and halfway down it; `\SECTION{id}` names it.

### Changed

- Section headers in the song PDF show the section's first and last bar
  (`17 → 28`), right-aligned in small type.

### Fixed

- Chord-chart cells no longer change size from song to song. The chart is
  scaled against a fixed two-column frame, the title and author shrink to fit
  it, and `add.tikz` drawings no longer count toward the chart's size.
- The `\include`s of a `.ly` reached only through `files.mp3` (not listed
  under `files.lilypond`) were never mounted, so its MIDI render failed.
- `songbook.ily` is compiled into the binary and always installed in the
  sandbox, so a corpus without its own copy still gets the macros. A
  `songbook.ily` beside the corpus `settings.yml` replaces it.
- `songbookBeatMarks` marks beats 1 and 3 with a square and 2 and 4 with a
  cross.
- `make install` installs the binary; `make run` passes the drum pattern
  library only when it exists.

## [0.0.37]

### Fixed

- A `.ly` file reached only through `\include` was never mounted into the
  sandbox, so lilypond could not resolve the include and the build died with
  "unknown escaped string" on whatever the included file defined. The includes
  are now collected from the sources - the sandbox is still empty when the
  graph is built, so scanning it there finds nothing - and mounted as source
  nodes, without the lytex/PDF chain they have no `\score` for.

## [0.0.36]

### Added

- Books. A yaml file in the directory given by the new `--books-srcdir` lists a
  `name`, its `tags` and its `songs`, each song given by its normalized
  `<author>--@--<title>` stem. Each book builds a `main.tex` in the sandbox that
  imports the listed songs from their own build directories, and delivers
  `pdf/book-<name>.pdf`. A book whose songs are not all built (because of
  `--pattern`) is skipped; a book listing an unknown song fails the build. No
  books are built when `--books-srcdir` is not given.
- A book opens on its setlist, whose entries link to the page their song starts
  on, and every song page carries `précédent` / `table des matières` /
  `suivant` links in the footer. The first and last songs grey out the link
  they have no target for.

### Changed

- `--srcdir` is now `--songs-srcdir`, next to the new `--books-srcdir`. The
  lambda request field is `songs_srcdir`, still accepting `srcdir` as an alias.

### Fixed

- lualatex was never run a second time: the rerun check looked for "Rerun to get
  the references right", which LaTeX does not print — it says "Rerun to get
  cross-references right". Any document needing two passes was delivered a pass
  stale, which is how a book came out numbering its pages against the previous
  build.

## [0.0.33]

### Fixed

- A negative click time lost its sign when the minutes were zero: `-00:02.00`
  parsed as `+2.0` seconds, because the minutes field was parsed on its own and
  `-00` is just `-0`. The sign is now taken from the whole string before
  splitting, so `-00:02.00` is -2 s. A sign inside the value (`00:-02.00`) is
  rejected with a message pointing at the `-mm:ss.ms` form, and surrounding
  whitespace is trimmed.

- `clicks.yml` did not depend on `song.yml` in the build graph, so a
  `section_id` anchor kept its old bar number after the section it points at
  changed length. The edge is now declared and the clicks are re-pinned on
  rebuild.

### Removed

- `go.sh`, `start-aws.sh` and `start.json`: local dev helpers that still
  referred to the `software/` directory and the `legendary-memory` repository,
  neither of which exists.

## [0.0.32]

### Added

- `.mp3` renders for sections listed under `files.mp3` in `song.yml` (the field
  was previously `files.wav` and unused). A new `MidiOfLilypond` node runs
  `lilypond` directly on `<stem>.ly` to get a predictable `<stem>.midi`, and a
  new `Mp3Render` node synthesises it with `fluidsynth` and encodes it with
  `ffmpeg`. The soundfont is taken from `soundfont:` in `settings.yml`, then
  `BAND_SONGBOOK_SOUNDFONT`, then the usual system paths. Declaring a render
  does not put the score in the PDF. Renders are delivered under
  `mp3-renders/<author>--@--<title>-<section>.mp3`.

- A standalone cropped PDF per LilyPond file, so a single score can be shown on
  its own. A new `PdfOfLilypond` node runs `lilypond -dcrop`, and the result is
  delivered under `pdf-snippets/<author>--@--<title>-<section>.pdf`.

- `clicks-def.yml` entries can reference a section instead of a bar:
  `section_id: couplet1` resolves to that section's first bar, so a click
  definition survives edits to the sections above it. `bar_number` and
  `section_id` are mutually exclusive, and exactly one of them is required.

- `\mypull`, `\mypulled` and `\myrelease` in `macros.ly`, for guitar
  articulation marks.

### Fixed

- Accented letters in a song's author or title no longer become `_` in delivered
  file names: they fold to their ASCII base, so "Noir Desir / Marlene" is
  `noir_desir--@--marlene` and not `noir_d_sir--@--marl_ne`. This renames
  existing delivery artifacts for songs with accents.

- `files.lilypond` in `song.yml` was parsed but never used. Declared `.ly` files
  are now build dependencies, so editing one rebuilds the song PDF even when no
  tex file references it. A declared file that does not exist is skipped with a
  warning.

- The chord chart printed a hardcoded `140 BPM` instead of the song's own
  `info.tempo`.

### Changed

- `\basecouplet` no longer reserves a line of vertical space for a section whose
  lyrics file is empty, so a run of lyric-less sections stacks tightly

- `clicks-def.yml` is rejected when its times are not strictly increasing. The
  click times are interpolated from the gap between consecutive entries, so
  equal or backwards times used to produce garbage silently.

- The `files.wav` field in `song.yml` is now `files.mp3`. Nothing ever read
  `files.wav`, so this only matters for song.yml files that set it.

## [0.0.1] - 2026-02-03

### Added

- Initial release
- Song discovery from `song.yml` files
- Chord parsing with support for major, minor, 7th, dim, sus2, sus4 chords
- Sharp and flat accidentals
- Rest notation (HRest)
- Repeat markers (x2, x3, etc.)
- LaTeX/TikZ chord chart generation
- LilyPond integration via `lilypond-book`
- Incremental builds using yamake dependency graph
- Fuzzy pattern matching for selective builds
- Configurable section colors via `settings.yml`
- Handlebars templating for LaTeX output
- Support for lyrics files per section
- Mermaid graph output for build visualization

### Dependencies

- yamake v0.1.9 for build system
- handlebars for templating
- serde/serde_yaml for configuration parsing
- argh for CLI argument parsing

[Unreleased]: https://github.com/laurentcarrie/songbook-lambda/compare/band-songbook-v0.0.33...HEAD
[0.0.33]: https://github.com/laurentcarrie/songbook-lambda/releases/tag/band-songbook-v0.0.33
[0.0.32]: https://github.com/laurentcarrie/songbook-lambda/releases/tag/band-songbook-v0.0.32
[0.0.1]: https://github.com/laurentcarrie/songbook-lambda/releases/tag/band-songbook-v0.0.1
