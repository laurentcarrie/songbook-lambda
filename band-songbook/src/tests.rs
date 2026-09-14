use crate::chords::parse::parse;
use crate::model::{BookItem, Song, WorldItem};
use crate::nodes::{ClickDef, ClickYml, LilypondFile, PdfFile, SongYml, TexFile};
use crate::{books_of_srcdir, discover, discover_books};
use crate::{make_all, world_of_srcdir};
use std::path::{Path, PathBuf};
use yamake::model::{G, GNode};

#[test]
fn test_read_song_from_yaml() {
    let yaml_content = std::fs::read_to_string("tests/data/PJHarvey/Dress/song.yml")
        .expect("Failed to read song.yml");
    let song: Song = serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML");

    assert_eq!(song.info.author, "P.J. Harvey");
    assert_eq!(song.info.title, "Dress");
    assert_eq!(song.info.tempo, 96);
}

#[test]
fn test_yamake_build_song() {
    let srcdir = PathBuf::from("tests/data");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");

    let mut g = G::new(srcdir, sandbox.path().to_path_buf());

    // Add settings.yml as root node so it gets copied to sandbox
    let colors_node = TexFile::new(PathBuf::from("settings.yml"));
    let _ = g.add_root_node(colors_node);

    // SongYml is a root node (source file)
    let song: Song = serde_yaml::from_str(
        &std::fs::read_to_string("tests/data/PJHarvey/Dress/song.yml").unwrap(),
    )
    .unwrap();
    let song_node = SongYml::new(PathBuf::from("PJHarvey/Dress/song.yml"), song);
    let song_idx = g.add_root_node(song_node).expect("Failed to add song node");

    // body.tex is a root node (source file from srcdir)
    let body_node = TexFile::new(PathBuf::from("PJHarvey/Dress/body.tex"));
    let _ = g.add_root_node(body_node);

    // add.tikz is a root node too: song.tikz inputs it
    let add_tikz_node = TexFile::new(PathBuf::from("PJHarvey/Dress/add.tikz"));
    let _ = g.add_root_node(add_tikz_node);

    // Pre-add PdfFile with Initial status so it goes through build loop
    // (yamake marks expanded nodes as "mounted" which skips build)
    // Add edge from SongYml so PdfFile isn't treated as a root node
    let pdf_node = PdfFile::new(PathBuf::from("PJHarvey/Dress/main.pdf"));
    let pdf_idx = g.add_node(pdf_node).expect("Failed to add pdf node");
    g.add_edge(song_idx, pdf_idx);

    let success = g.make();
    assert!(success, "Build should succeed");

    // Verify the file was mounted to sandbox
    let mounted_path = sandbox.path().join("PJHarvey/Dress/song.yml");
    assert!(
        mounted_path.exists(),
        "song.yml should be mounted in sandbox"
    );

    // Verify main.tex was created by expand
    let tex_path = sandbox.path().join("PJHarvey/Dress/main.tex");
    assert!(tex_path.exists(), "main.tex should be created in sandbox");

    // Verify main.pdf was built
    let pdf_path = sandbox.path().join("PJHarvey/Dress/main.pdf");
    assert!(pdf_path.exists(), "main.pdf should be built in sandbox");
}

#[test]
fn test_yamake_build_song_with_lilypond() {
    let srcdir = PathBuf::from("tests/data");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");

    let mut g = G::new(srcdir, sandbox.path().to_path_buf());

    // Add settings.yml as root node so it gets copied to sandbox
    let colors_node = TexFile::new(PathBuf::from("settings.yml"));
    let _ = g.add_root_node(colors_node);

    // songbook.ily is the corpus library that the generated macros.ly includes
    // as ../../songbook.ily; make_all mirrors it into the songs sandbox, and
    // here the sandbox root plays that part.
    let library_node = LilypondFile::new(PathBuf::from("songbook.ily"));
    let _ = g.add_root_node(library_node);

    // mademoiselle_K/ca_me_vexe - has lilypond files
    let song: Song = serde_yaml::from_str(
        &std::fs::read_to_string("tests/data/mademoiselle_K/ca_me_vexe/song.yml").unwrap(),
    )
    .unwrap();
    let song_node = SongYml::new(PathBuf::from("mademoiselle_K/ca_me_vexe/song.yml"), song);
    let song_idx = g.add_root_node(song_node).expect("Failed to add song node");

    let body_node = TexFile::new(PathBuf::from("mademoiselle_K/ca_me_vexe/body.tex"));
    let _ = g.add_root_node(body_node);

    let add_tikz_node = TexFile::new(PathBuf::from("mademoiselle_K/ca_me_vexe/add.tikz"));
    let _ = g.add_root_node(add_tikz_node);

    // Add clicks-def as root node and clicks.yml as build node (has_clicks is true)
    let clicks_def_node = ClickDef::new(
        PathBuf::from("mademoiselle_K/ca_me_vexe/clicks-def.yml"),
        Path::new("tests/data"),
    )
    .expect("load clicks definition");
    let def_idx = g.add_root_node(clicks_def_node).expect("add clicks-def");

    let clicks_node = ClickYml::new(PathBuf::from("mademoiselle_K/ca_me_vexe/clicks.yml"));
    let clicks_idx = g.add_node(clicks_node).expect("add clicks.yml");
    g.add_edge(def_idx, clicks_idx);

    // Add lyrics files as root nodes (like make_all does)
    let lyrics_files = [
        "intro",
        "couplet1",
        "refrain1",
        "couplet2a",
        "couplet2b",
        "couplet2c",
        "refrain2",
        "couplet3a",
        "couplet3b",
        "refrain3",
        "interlude",
        "final",
    ];
    for name in lyrics_files {
        let lyrics_path = PathBuf::from(format!("mademoiselle_K/ca_me_vexe/lyrics/{name}.tex"));
        let lyrics_node = TexFile::new(lyrics_path);
        let _ = g.add_root_node(lyrics_node);
    }

    let pdf_node = PdfFile::new(PathBuf::from("mademoiselle_K/ca_me_vexe/main.pdf"));
    let pdf_idx = g.add_node(pdf_node).expect("Failed to add pdf node");
    g.add_edge(song_idx, pdf_idx);

    let success = g.make();
    assert!(success, "Build should succeed");
    // Verify main.pdf was built
    let pdf_path = sandbox.path().join("mademoiselle_K/ca_me_vexe/main.pdf");
    assert!(
        pdf_path.exists(),
        "mademoiselle_K/ca_me_vexe/main.pdf should be built in sandbox"
    );

    // Verify interlude.ly node exists with tag 'lilypond'
    let interlude_ly_found = g.g.node_indices().any(|idx| {
        let node = &g.g[idx];
        node.pathbuf() == PathBuf::from("mademoiselle_K/ca_me_vexe/interlude.ly")
            && node.tag() == "lilypond"
    });
    assert!(
        interlude_ly_found,
        "interlude.ly node should exist with tag 'lilypond'"
    );

    // Verify interlude.ly is in the predecessor tree of main.pdf
    let predecessors = g.root_predecessors(pdf_idx);
    let interlude_is_predecessor = predecessors
        .iter()
        .any(|&idx| g.g[idx].pathbuf() == PathBuf::from("mademoiselle_K/ca_me_vexe/interlude.ly"));
    assert!(
        interlude_is_predecessor,
        "interlude.ly should be a predecessor of main.pdf"
    );

    // Verify interlude.output/interlude.tex was generated by lilypond-book
    let interlude_tex_path = sandbox
        .path()
        .join("mademoiselle_K/ca_me_vexe/interlude.output/interlude.tex");
    assert!(
        interlude_tex_path.exists(),
        "interlude.output/interlude.tex should be generated"
    );

    // Verify macros.ly is in the predecessor tree of main.pdf (via interlude.ly \include)
    let macros_is_predecessor = predecessors
        .iter()
        .any(|&idx| g.g[idx].pathbuf() == PathBuf::from("mademoiselle_K/ca_me_vexe/macros.ly"));
    assert!(
        macros_is_predecessor,
        "macros.ly should be a predecessor of main.pdf"
    );
}

#[test]
fn test_yamake_build_pdf() {
    let srcdir = PathBuf::from("tests/data");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");

    let mut g = G::new(srcdir, sandbox.path().to_path_buf());

    // Add TexFile as root node
    let tex_node = TexFile::new(PathBuf::from("tex/hello.tex"));
    let tex_idx = g.add_root_node(tex_node).expect("Failed to add tex node");

    // Add PdfFile as build node
    let pdf_node = PdfFile::new(PathBuf::from("tex/hello.pdf"));
    let pdf_idx = g.add_node(pdf_node).expect("Failed to add pdf node");

    // Add edge: tex -> pdf
    g.add_edge(tex_idx, pdf_idx);

    let success = g.make();
    assert!(success, "Build should succeed");

    // Verify the PDF was created
    let pdf_path = sandbox.path().join("tex/hello.pdf");
    assert!(pdf_path.exists(), "hello.pdf should be created in sandbox");
}

#[test]
fn test_discover() {
    let mut songs = discover(Path::new("tests/data"));
    songs.sort();

    assert_eq!(songs.len(), 2);
    assert!(songs[0].ends_with("PJHarvey/Dress/song.yml"));
    assert!(songs[1].ends_with("mademoiselle_K/ca_me_vexe/song.yml"));
}

#[test]
fn test_make_all() {
    let srcdir = Path::new("tests/data");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let world = world_of_srcdir(srcdir);

    let (success, _g) = make_all(
        srcdir,
        None,
        sandbox.path(),
        Some(Path::new("tests/data/settings.yml")),
        None,
        &[],
        &world,
    );
    assert!(success, "make_all should succeed");

    // Verify PDFs were created for both songs
    let pdf1 = sandbox.path().join("songs/PJHarvey/Dress/main.pdf");
    assert!(
        pdf1.exists(),
        "songs/PJHarvey/Dress/main.pdf should be created"
    );

    let pdf2 = sandbox
        .path()
        .join("songs/mademoiselle_K/ca_me_vexe/main.pdf");
    assert!(
        pdf2.exists(),
        "songs/mademoiselle_K/ca_me_vexe/main.pdf should be created"
    );
}

#[test]
fn test_make_all_with_broken_song() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");
    let srcdir = tmpdir.path();

    // Create a valid song
    let valid_dir = srcdir.join("good/song");
    std::fs::create_dir_all(&valid_dir).expect("create dir");
    std::fs::write(
        valid_dir.join("song.yml"),
        r#"
files:
  lilypond: []
  tex: []
  mp3: []
info:
  title: Good Song
  author: Good Author
  tempo: 120
meta:
  date: null
  digest: null
structure: []
"#,
    )
    .expect("write valid song.yml");

    // Create a broken song
    let broken_dir = srcdir.join("bad/song");
    std::fs::create_dir_all(&broken_dir).expect("create dir");
    std::fs::write(broken_dir.join("song.yml"), "this is broken").expect("write broken song.yml");

    let world = world_of_srcdir(srcdir);
    assert_eq!(world.items.len(), 2);

    // One should be an error
    let errors: Vec<_> = world
        .items
        .iter()
        .filter(|(_, item)| matches!(item, WorldItem::Error(_)))
        .collect();
    assert_eq!(errors.len(), 1, "should have exactly one error");

    // make_all should fail
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let (success, _g) = make_all(srcdir, None, sandbox.path(), None, None, &[], &world);
    assert!(!success, "make_all should fail with a broken song");
}

#[test]
fn test_pdf_scan() {
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");

    // Create a tex file with \input instructions
    let tex_dir = sandbox.path().join("song");
    std::fs::create_dir_all(&tex_dir).expect("Failed to create dir");

    let tex_content = r#"\documentclass{article}
\begin{document}
\input{intro.tex}
\input{verse1.tex}
\input{chorus.tex}
\end{document}
"#;
    std::fs::write(tex_dir.join("main.tex"), tex_content).expect("Failed to write tex file");

    let tex_node = TexFile::new(PathBuf::from("song/main.tex"));
    let pdf_node = PdfFile::new(PathBuf::from("song/main.pdf"));

    let predecessors: Vec<&(dyn GNode + Send + Sync)> = vec![&tex_node];
    let (success, inputs) = pdf_node.scan(sandbox.path(), &predecessors);

    assert!(success);
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0], PathBuf::from("song/intro.tex"));
    assert_eq!(inputs[1], PathBuf::from("song/verse1.tex"));
    assert_eq!(inputs[2], PathBuf::from("song/chorus.tex"));
}

#[test]
fn test_parse_chords() {
    use crate::chords::model::{Alteration, BarItem, Repeat, Rest};

    let input = "Em | Em|C7|G|Am Bm7|HRest|Csm7|Edim|Bsus4|Cssus2|x2";
    let result = parse(input).unwrap();
    assert_eq!(result.bars.len(), 10);
    assert_eq!(result.repeat, Repeat { n: 2 });

    // First bar: Em (E minor)
    assert_eq!(result.bars[0].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[0].items[0] {
        assert_eq!(chord.name, "E");
        assert!(chord.minor);
        assert_eq!(chord.alteration, Alteration::None);
    } else {
        panic!("Expected Chord");
    }

    // Second bar: Em (E minor)
    assert_eq!(result.bars[1].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[1].items[0] {
        assert_eq!(chord.name, "E");
        assert!(chord.minor);
    } else {
        panic!("Expected Chord");
    }

    // Third bar: C7 (C seventh)
    assert_eq!(result.bars[2].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[2].items[0] {
        assert_eq!(chord.name, "C");
        assert!(!chord.minor);
        assert_eq!(chord.alteration, Alteration::Seven);
    } else {
        panic!("Expected Chord");
    }

    // Fourth bar: G (G major)
    assert_eq!(result.bars[3].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[3].items[0] {
        assert_eq!(chord.name, "G");
        assert!(!chord.minor);
        assert_eq!(chord.alteration, Alteration::None);
    } else {
        panic!("Expected Chord");
    }

    // Fifth bar: Am Bm7 (two chords)
    assert_eq!(result.bars[4].items.len(), 2);
    // Am (A minor)
    if let BarItem::Chord(chord) = &result.bars[4].items[0] {
        assert_eq!(chord.name, "A");
        assert!(chord.minor);
        assert_eq!(chord.alteration, Alteration::None);
    } else {
        panic!("Expected Chord");
    }
    // Bm7 (B minor seventh)
    if let BarItem::Chord(chord) = &result.bars[4].items[1] {
        assert_eq!(chord.name, "B");
        assert!(chord.minor);
        assert_eq!(chord.alteration, Alteration::Seven);
    } else {
        panic!("Expected Chord");
    }

    // Sixth bar: HRest
    assert_eq!(result.bars[5].items.len(), 1);
    assert_eq!(result.bars[5].items[0], BarItem::Rest(Rest { duration: 1 }));

    // Seventh bar: Csm7 (C sharp minor seventh)
    assert_eq!(result.bars[6].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[6].items[0] {
        assert_eq!(chord.name, "C");
        assert_eq!(chord.accidental, crate::chords::model::Accidental::Sharp);
        assert!(chord.minor);
        assert_eq!(chord.alteration, Alteration::Seven);
    } else {
        panic!("Expected Chord");
    }

    // Eighth bar: Edim (E diminished)
    assert_eq!(result.bars[7].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[7].items[0] {
        assert_eq!(chord.name, "E");
        assert!(!chord.minor);
        assert_eq!(chord.alteration, Alteration::Dim);
    } else {
        panic!("Expected Chord");
    }

    // Ninth bar: Bsus4 (B sus4)
    assert_eq!(result.bars[8].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[8].items[0] {
        assert_eq!(chord.name, "B");
        assert!(!chord.minor);
        assert_eq!(chord.alteration, Alteration::Sus4);
    } else {
        panic!("Expected Chord");
    }

    // Tenth bar: Cssus2 (C sharp sus2)
    assert_eq!(result.bars[9].items.len(), 1);
    if let BarItem::Chord(chord) = &result.bars[9].items[0] {
        assert_eq!(chord.name, "C");
        assert_eq!(chord.accidental, crate::chords::model::Accidental::Sharp);
        assert!(!chord.minor);
        assert_eq!(chord.alteration, Alteration::Sus2);
    } else {
        panic!("Expected Chord");
    }
}

#[test]
fn test_make_all_with_pattern() {
    let srcdir = Path::new("tests/data");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let world = world_of_srcdir(srcdir);

    // Pattern "madkvex" should match "Mademoiselle K Ca me vexe"
    let (success, _g) = make_all(
        srcdir,
        Some(Path::new("tests/data/books")),
        sandbox.path(),
        Some(Path::new("tests/data/settings.yml")),
        Some("madkvex"),
        &[],
        &world,
    );
    assert!(success, "make_all with pattern should succeed");

    // Only Ca Me Vexe should be built (matches "Mademoiselle K Ca me vexe")
    let pdf_vexe = sandbox
        .path()
        .join("songs/mademoiselle_K/ca_me_vexe/main.pdf");
    assert!(
        pdf_vexe.exists(),
        "songs/mademoiselle_K/ca_me_vexe/main.pdf should be created"
    );

    // PJHarvey/Dress should NOT be built (doesn't match pattern)
    let pdf_dress = sandbox.path().join("songs/PJHarvey/Dress/main.pdf");
    assert!(
        !pdf_dress.exists(),
        "songs/PJHarvey/Dress/main.pdf should NOT be created"
    );
}

#[test]
fn test_parse_invalid_chord() {
    use crate::chords::parse::ParseError;

    let invalid_inputs = ["X", "Am+", "Ax"];
    for input in invalid_inputs {
        let result = parse(input);
        assert!(result.is_err(), "Expected error for input: {input}");
        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidChord(input.to_string())
        );
    }
}

#[tokio::test]
async fn test_make_all_with_storage_local() {
    use crate::make_all_with_storage;

    let srcdir = "tests/data";
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let settings = "tests/data/settings.yml";
    let delivery = tempfile::tempdir().expect("Failed to create delivery dir");
    let delivery_path = delivery.path().to_str().unwrap();

    let result = make_all_with_storage(
        srcdir,
        Some("tests/data/books"),
        sandbox.path(),
        Some(settings),
        None,
        delivery_path,
        &[],
    )
    .await;

    assert!(result.is_ok(), "make_all_with_storage should succeed");
    let (success, _g) = result.unwrap();
    assert!(success, "Build should succeed");

    // Verify PDFs were created
    let pdf1 = sandbox.path().join("songs/PJHarvey/Dress/main.pdf");
    assert!(
        pdf1.exists(),
        "songs/PJHarvey/Dress/main.pdf should be created"
    );

    let pdf2 = sandbox
        .path()
        .join("songs/mademoiselle_K/ca_me_vexe/main.pdf");
    assert!(
        pdf2.exists(),
        "songs/mademoiselle_K/ca_me_vexe/main.pdf should be created"
    );
}

#[test]
fn test_make_all_has_clicks_and_has_mp3() {
    // Create a temp srcdir with a song that has both has_clicks and has_mp3
    let srcdir = tempfile::tempdir().expect("Failed to create srcdir");
    let song_dir = srcdir.path().join("TestArtist/ClickSong");
    std::fs::create_dir_all(&song_dir).expect("create song dir");

    std::fs::write(
        song_dir.join("song.yml"),
        r#"
files:
  lilypond: []
  tex: []
  mp3: []
  has_clicks: true
  has_mp3: true
info:
  title: Click Song
  author: Test Artist
  tempo: 120
meta:
  date: null
  digest: null
structure: []
"#,
    )
    .expect("write song.yml");

    std::fs::write(
        song_dir.join("body.tex"),
        "\\input{song.tikz}\n\\newpage\n\\songlyrics\n",
    )
    .expect("write body.tex");

    std::fs::write(song_dir.join("add.tikz"), "% no extra drawings\n").expect("write add.tikz");

    // Provide clicks.mp3, clicks-def.yml, and song.mp3
    std::fs::copy("tests/data/click/clicks.mp3", song_dir.join("clicks.mp3"))
        .expect("copy clicks.mp3");
    std::fs::write(
        song_dir.join("clicks-def.yml"),
        "clicks:\n- bar_number: 1\n  beat_in_bar_number: 1\n  time: \"0:0.0\"\n  description: start\n- bar_number: 1\n  beat_in_bar_number: 3\n  time: \"0:0.5\"\n  description: end\n",
    )
    .expect("write clicks-def.yml");
    std::fs::copy("tests/data/click/clicks.mp3", song_dir.join("song.mp3")).expect("copy song.mp3");

    std::fs::copy(
        "tests/data/settings.yml",
        srcdir.path().join("settings.yml"),
    )
    .expect("copy settings.yml");

    let sandbox = tempfile::tempdir().expect("Failed to create sandbox");
    let world = world_of_srcdir(srcdir.path());

    let (success, g) = make_all(
        srcdir.path(),
        None,
        sandbox.path(),
        Some(Path::new(&srcdir.path().join("settings.yml"))),
        None,
        &[],
        &world,
    );
    assert!(success, "make_all should succeed");

    // Verify clicks.yml was mounted in sandbox
    let clicks_yml_path = sandbox.path().join("songs/TestArtist/ClickSong/clicks.yml");
    assert!(
        clicks_yml_path.exists(),
        "clicks.yml should be mounted in sandbox"
    );

    // Verify song.mp3 was mounted in sandbox
    let song_mp3_path = sandbox.path().join("songs/TestArtist/ClickSong/song.mp3");
    assert!(
        song_mp3_path.exists(),
        "song.mp3 should be mounted in sandbox"
    );

    // Verify clicks.yml node exists with correct tag
    let clicks_yml_found = g.g.node_indices().any(|idx| {
        let node = &g.g[idx];
        node.pathbuf() == PathBuf::from("TestArtist/ClickSong/clicks.yml")
            && node.tag() == "clicks.yml"
    });
    assert!(
        clicks_yml_found,
        "clicks.yml node should exist with tag 'clicks.yml'"
    );
}

/// Integration test for S3 storage.
/// Run with: AWS_PROFILE=zik-laurent cargo test test_make_all_with_s3 -- --ignored --nocapture
#[tokio::test]
#[ignore]
async fn test_make_all_with_s3() {
    use crate::make_all_with_storage;

    let srcdir = "s3://zik-laurent/songs";
    let settings = "s3://zik-laurent/songs/settings.yml";
    let delivery = "s3://zik-laurent/delivery";
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");

    let result = make_all_with_storage(
        srcdir,
        None,
        sandbox.path(),
        Some(settings),
        None,
        delivery,
        &[],
    )
    .await;

    match &result {
        Ok((success, _g)) => {
            println!("Build completed, success: {success}");
            assert!(*success, "Build should succeed");
        }
        Err(e) => {
            panic!("make_all_with_storage failed: {e}");
        }
    }
}

#[test]
fn test_discover_books() {
    let books = discover_books(Path::new("tests/data/books"));
    assert_eq!(books.len(), 1);
    assert!(books[0].ends_with("books/test-book.yml"));

    let books = books_of_srcdir(Path::new("tests/data/books"));
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].0, PathBuf::from("test-book.yml"));
    let book = match &books[0].1 {
        BookItem::Book(b) => b,
        BookItem::Error(e) => panic!("book should parse: {e}"),
    };
    assert_eq!(book.name, "Test Book");
    assert_eq!(book.tags, vec!["test", "rock"]);
    assert_eq!(book.songs.len(), 2);
    assert_eq!(book.file_stem_of_book(), "book-test_book");
}

#[test]
fn test_make_all_with_unknown_song_in_book() {
    let tmpdir = tempfile::tempdir().expect("Failed to create temp dir");
    let srcdir = tmpdir.path();

    std::fs::create_dir_all(srcdir.join("books")).expect("create books dir");
    std::fs::write(
        srcdir.join("books/empty.yml"),
        "name: Empty\nsongs:\n  - nobody--@--nothing\n",
    )
    .expect("write book");

    let books_dir = srcdir.join("books");
    let mut world = world_of_srcdir(srcdir);
    world.books = books_of_srcdir(&books_dir);
    assert_eq!(world.books.len(), 1);

    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let (success, _g) = make_all(
        srcdir,
        Some(&books_dir),
        sandbox.path(),
        None,
        None,
        &[],
        &world,
    );
    assert!(
        !success,
        "make_all should fail on a book with an unknown song"
    );
}

#[test]
fn test_make_all_book() {
    let srcdir = Path::new("tests/data");
    let books_srcdir = Path::new("tests/data/books");
    let sandbox = tempfile::tempdir().expect("Failed to create temp dir");
    let mut world = world_of_srcdir(srcdir);
    world.books = books_of_srcdir(books_srcdir);

    let (success, _g) = make_all(
        srcdir,
        Some(books_srcdir),
        sandbox.path(),
        Some(Path::new("tests/data/settings.yml")),
        None,
        &[],
        &world,
    );
    assert!(success, "make_all should succeed");

    // The book collates both songs of the test data
    let tex = sandbox.path().join("songs/books/test_book/main.tex");
    assert!(tex.exists(), "the book main.tex should be created");
    let content = std::fs::read_to_string(&tex).expect("read book main.tex");
    assert!(content.contains("\\import{../../PJHarvey/Dress/}{body.tex}"));
    assert!(content.contains("\\import{../../mademoiselle_K/ca_me_vexe/}{body.tex}"));

    let pdf = sandbox.path().join("songs/books/test_book/main.pdf");
    assert!(pdf.exists(), "the book main.pdf should be built");

    let delivered = sandbox.path().join("pdf/book-test_book.pdf");
    assert!(
        delivered.exists(),
        "pdf/book-test_book.pdf should be delivered"
    );
}
