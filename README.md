# band-songbook

A build system for generating PDF songbooks with chord charts and LilyPond music notation.

## Features

- Parses `song.yml` files defining song structure (chords, lyrics, sections)
- Generates LaTeX/TikZ chord charts
- Integrates LilyPond for music notation (tablature, scores)
- Incremental builds using the yamake dependency graph system
- Fuzzy pattern matching to build specific songs

## Installation

```bash
cargo install band-songbook
```

### Requirements

- **LuaLaTeX** - for PDF generation
- **LilyPond** with `lilypond-book` - for music notation (optional)

## Usage

```bash
band-songbook --songs-srcdir <SONGS_DIR> [--books-srcdir <BOOKS_DIR>] --sandbox <OUTPUT_DIR> [--settings <SETTINGS_FILE>] [--pattern <PATTERN>]
```

### Options

- `-s, --songs-srcdir` - Source directory containing `song.yml` files
- `-b, --books-srcdir` - Directory containing the book yaml files (optional; no books are built without it)
- `-o, --sandbox` - Output directory for generated files
- `-c, --settings` - Path to `settings.yml` for colors and configuration
- `-p, --pattern` - Fuzzy pattern to filter songs (e.g., "beatles" or "yesterday")

### Example

```bash
band-songbook --songs-srcdir ./songs --books-srcdir ./books --sandbox ./build --settings ./settings.yml
```

Build only songs matching "velvet":
```bash
band-songbook --songs-srcdir ./songs --sandbox ./build --pattern velvet
```

## Song Format

Each song is defined by a `song.yml` file:

```yaml
info:
  author: "Artist Name"
  title: "Song Title"
  tempo: 120

structure:
  - id: intro
    item:
      Chords:
        section_type: intro
        chords: "Am | G | F | E"

  - id: verse1
    item:
      Chords:
        section_type: couplet
        chords: "Am | G | C | F | Am | G | E | E"

  - id: chorus
    item:
      Chords:
        section_type: refrain
        chords: "F | G | Am | Am | F | G | C | C"
```

### Directory Structure

```
songs/
  artist_name/
    song_title/
      song.yml        # Song definition
      body.tex        # Main content template
      add.tikz        # Extra TikZ drawings, appended to the chord chart
      lyrics/
        intro.tex     # Lyrics for intro section
        verse1.tex    # Lyrics for verse1 section
        chorus.tex    # Lyrics for chorus section
      interlude.ly    # Optional LilyPond notation
```

## Book Format

A book collates songs into a single PDF. Each book is a yaml file in the
directory given by `--books-srcdir`:

```yaml
name: Rock Set
tags:
  - live
  - 2026
songs:
  - p_j__harvey--@--dress
  - mademoiselle_k--@--ca_me_vexe
```

- `name` - book name, printed on the first page and used for the delivered file
  name: `delivery/pdf/book-<name>.pdf` (normalized the same way as song files)
- `tags` - free-form tags for categorizing the book
- `songs` - the songs of the book, in the order they are collated, each given by
  its normalized stem `<author>--@--<title>`, the same name as the song PDF

Each listed song is imported from its own build directory, so a book is exactly
the concatenation of the song pages it lists, opening on a setlist whose entries
link to their song. Every song page carries `précédent` / `table des matières` /
`suivant` links in the footer. A book whose songs are not all
built (because of `--pattern`) is skipped; a book listing a song that does not
exist fails the build.

```
books/
  rock_set.yml      # Book definition, --books-srcdir books
songs/              # --songs-srcdir songs
  artist_name/
    song_title/
      song.yml
```

## License

MIT
