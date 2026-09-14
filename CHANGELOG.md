# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
