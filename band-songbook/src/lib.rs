/// Chord parsing, bar numbering, and glyph rendering.
pub mod chords;
/// Song discovery by scanning directories for `song.yml` files.
pub mod discover;
/// Handlebars template helpers and song-to-sequence conversion.
pub mod helpers;
/// Data model for songs, sections, chords, and drum sequences.
pub mod model;
/// Build graph nodes for the yamake dependency system.
pub mod nodes;
/// Build settings loaded from `settings.yml`.
pub mod settings;
/// S3 and local filesystem storage operations.
pub mod storage;

pub use discover::{discover, discover_books};
pub use nodes::PdfFile;

use model::{Book, BookItem, SectionItem, Song, World, WorldItem};
use nodes::bookyml::BOOKS_DIR;
use nodes::{BookSong, BookYml, ClickDef, ClickYml, CopyFile, SongYml, TexFile};
use object_store::ObjectStoreExt;
use std::path::{Path, PathBuf};

pub use yamake::model::G;
use yamake::model::GNode;

/// Returns all LilyPond files (.ly) referenced by a PdfFile node.
/// Scans the tex files to find \lyfile{}, \songly{}, and \include directives.
pub fn get_lilypond_files(
    pdf: &PdfFile,
    sandbox: &Path,
    predecessors: &[&(dyn GNode + Send + Sync)],
) -> Vec<std::path::PathBuf> {
    let (_, inputs, _) = pdf.scan_with_toplevel_ly(sandbox, predecessors);
    inputs
        .into_iter()
        .filter(|p| p.extension().map(|e| e == "ly").unwrap_or(false))
        .collect()
}

/// Checks if pattern is a subsequence of text (fuzzy match).
/// Each character in pattern must appear in text in order, but not necessarily contiguously.
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut text_chars = text.chars().peekable();
    for pat_char in pattern.chars() {
        loop {
            match text_chars.next() {
                Some(c) if c == pat_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Discovers all songs in the given directory and reads them into a [`World`].
///
/// Each discovered `song.yml` is read and parsed. Successfully parsed songs
/// are stored as [`WorldItem::Song`], failures as [`WorldItem::Error`]. The
/// returned world has no books; read those with [`books_of_srcdir`].
pub fn world_of_srcdir(srcdir: &Path) -> World {
    let song_paths = discover(srcdir);
    let items = song_paths
        .into_iter()
        .filter_map(|song_path| {
            let rel_path = song_path.strip_prefix(srcdir).ok()?.to_path_buf();
            let item = match std::fs::read_to_string(&song_path) {
                Ok(content) => match serde_yaml::from_str::<Song>(&content) {
                    Ok(song) => WorldItem::Song(Box::new(song)),
                    Err(e) => {
                        log::error!("Failed to parse {}: {e}", song_path.display());
                        WorldItem::Error(format!("Failed to parse {}: {e}", song_path.display()))
                    }
                },
                Err(e) => {
                    log::error!("Failed to read {}: {e}", song_path.display());
                    WorldItem::Error(format!("Failed to read {}: {e}", song_path.display()))
                }
            };
            Some((rel_path, item))
        })
        .collect();

    World {
        items,
        books: Vec::new(),
    }
}

/// Reads every yaml file of the given books directory into a book item.
///
/// Successfully parsed books are stored as [`BookItem::Book`], failures as
/// [`BookItem::Error`]. Each item is keyed by the file name of the book, which
/// is the name it is mounted under in the sandbox.
pub fn books_of_srcdir(books_srcdir: &Path) -> Vec<(PathBuf, BookItem)> {
    discover_books(books_srcdir)
        .into_iter()
        .filter_map(|book_path| {
            let file_name = PathBuf::from(book_path.file_name()?);
            let item = match std::fs::read_to_string(&book_path) {
                Ok(content) => match serde_yaml::from_str::<Book>(&content) {
                    Ok(book) => BookItem::Book(Box::new(book)),
                    Err(e) => {
                        log::error!("Failed to parse {}: {e}", book_path.display());
                        BookItem::Error(format!("Failed to parse {}: {e}", book_path.display()))
                    }
                },
                Err(e) => {
                    log::error!("Failed to read {}: {e}", book_path.display());
                    BookItem::Error(format!("Failed to read {}: {e}", book_path.display()))
                }
            };
            Some((file_name, item))
        })
        .collect()
}

/// Returns a sorted, deduplicated list of all tags found across all songs in the world.
pub fn tags_of_world(world: &World) -> Vec<String> {
    let mut tags: Vec<String> = world
        .items
        .iter()
        .filter_map(|(_, item)| match item {
            WorldItem::Song(song) => Some(&song.info.tags),
            WorldItem::Error(_) => None,
        })
        .flatten()
        .cloned()
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// Builds every song of the world, and every book that collates them.
/// Returns (success, graph) where success is true if all builds succeeded.
/// If settings_path is provided, it will be copied to sandbox/settings.yml.
/// If pattern is provided, only songs matching the pattern will be built.
/// `books_srcdir` is where the book yaml files of the world are read from;
/// they are copied into the sandbox next to the directories they build in.
pub fn make_all(
    songs_srcdir: &Path,
    books_srcdir: Option<&Path>,
    sandbox: &Path,
    settings_path: Option<&Path>,
    pattern: Option<&str>,
    drum_patterns_dirs: &[PathBuf],
    world: &World,
) -> (bool, G) {
    let srcdir = songs_srcdir;
    let songs_sandbox = sandbox.join("songs");
    let mut g = G::new(srcdir.to_path_buf(), songs_sandbox.clone());

    // Check that lualatex is available
    if std::process::Command::new("lualatex")
        .arg("--help")
        .output()
        .is_err()
    {
        log::error!("lualatex is not available. Please install TeX Live or similar.");
        return (false, g);
    }

    // Create songs directory
    if let Err(e) = std::fs::create_dir_all(&songs_sandbox) {
        log::error!("Failed to create songs directory: {e}");
    }

    // Copy settings.yml to songs sandbox if provided
    if let Some(settings) = settings_path {
        let dest = songs_sandbox.join("settings.yml");
        if let Err(e) = std::fs::copy(settings, &dest) {
            log::error!("Failed to copy settings.yml to sandbox: {e}");
        }
    }

    // songbook.ily is the corpus-wide LilyPond library: a real versioned file
    // beside settings.yml, unlike the generated macros.ly. Mirroring it here
    // means `\include "../../songbook.ily"` resolves to the same content from
    // songs/<artist>/<song>/ and from its sandbox copy - so a .ly compiles in
    // the editor with the real macros rather than a stand-in. Absent is fine:
    // a corpus that does not use the library simply never includes it.
    {
        let src = songs_srcdir.join("songbook.ily");
        if src.is_file() {
            let dest = songs_sandbox.join("songbook.ily");
            if let Err(e) = std::fs::copy(&src, &dest) {
                log::error!("Failed to copy songbook.ily to sandbox: {e}");
            }
        }
    }

    if world.items.is_empty() && world.books.is_empty() {
        return (true, g);
    }

    // Check for any errors in the world
    for (rel_path, item) in &world.items {
        if let WorldItem::Error(msg) = item {
            log::error!("{}: {msg}", rel_path.display());
            return (false, g);
        }
    }
    for (rel_path, item) in &world.books {
        if let BookItem::Error(msg) = item {
            log::error!("{}: {msg}", rel_path.display());
            return (false, g);
        }
    }

    // Create pdf directory for copied PDFs
    let pdf_dir = sandbox.join("pdf");
    if let Err(e) = std::fs::create_dir_all(&pdf_dir) {
        log::error!("Failed to create pdf directory: {e}");
    }

    // Create tempo directory for copied strudel HTML files
    let tempo_dir = sandbox.join("tempo");
    if let Err(e) = std::fs::create_dir_all(&tempo_dir) {
        log::error!("Failed to create tempo directory: {e}");
    }

    // Create pdf-lyrics directories for copied lyrics PDFs (1-column and 2-column)
    // and mp3/clicks directories for audio delivery
    for dir in [
        "pdf-lyrics-1-column",
        "pdf-lyrics-2-column",
        "mp3",
        "mp3-renders",
        "mp3-with-clicks",
        "clicks",
        "pdf-snippets",
    ] {
        let sub_dir = sandbox.join(dir);
        if let Err(e) = std::fs::create_dir_all(&sub_dir) {
            log::error!("Failed to create {dir} directory: {e}");
        }
    }

    // Directory and PDF node of every song that made it into the graph, by
    // normalized stem, so the books below can collate them.
    let mut built_songs = std::collections::HashMap::new();

    for (rel_path, item) in &world.items {
        let song = match item {
            WorldItem::Song(s) => s,
            WorldItem::Error(_) => unreachable!(), // errors already checked above
        };

        let parent_dir = rel_path.parent().unwrap_or(Path::new(""));

        // Filter by pattern if provided (fuzzy match against author + title)
        if let Some(pat) = pattern {
            let search_str = format!("{} {}", song.info.author, song.info.title).to_lowercase();
            if !fuzzy_match(&search_str, &pat.to_lowercase()) {
                continue;
            }
        }

        // Create SongYml node
        let song_node = SongYml::new(rel_path.clone(), *song.clone())
            .with_drum_patterns_dirs(drum_patterns_dirs.to_vec())
            .with_srcdir(srcdir);
        let song_idx = match g.add_root_node(song_node) {
            Ok(idx) => idx,
            Err(_) => continue,
        };

        // Add body.tex as root node (source file from srcdir, not generated)
        let body_path = parent_dir.join("body.tex");
        let body_node = TexFile::new(body_path);
        let _ = g.add_root_node(body_node);

        // Add add.tikz as root node: song-specific TikZ drawings, \input at the
        // end of the generated song.tikz, so every song must have one
        let add_tikz_path = parent_dir.join("add.tikz");
        let add_tikz_node = TexFile::new(add_tikz_path);
        let _ = g.add_root_node(add_tikz_node);

        // Add clicks-def and clicks.yml nodes if has_clicks is true
        if song.files.has_clicks {
            let clicks_def_path = parent_dir.join("clicks-def.yml");
            match ClickDef::new(clicks_def_path.clone(), srcdir) {
                Ok(def_node) => {
                    let def_idx = match g.add_root_node(def_node) {
                        Ok(idx) => idx,
                        Err(_) => continue,
                    };
                    let clicks_yml_path = parent_dir.join("clicks.yml");
                    let clicks_node = ClickYml::new(clicks_yml_path);
                    if let Ok(clicks_idx) = g.add_node(clicks_node) {
                        g.add_edge(def_idx, clicks_idx);
                        // clicks.yml also depends on song.yml: `section_id`
                        // anchors resolve to bar numbers through the song
                        // structure, so editing a section's length must
                        // re-pin the clicks.
                        g.add_edge(song_idx, clicks_idx);
                    }
                }
                Err(e) => {
                    log::error!("{e}");
                    return (false, g);
                }
            }
        }

        // Add lyrics files as root nodes for each Chords and Ref section
        for item in &song.structure {
            match &item.item {
                SectionItem::Chords(_) | SectionItem::Ref(_) => {
                    let lyrics_path = parent_dir.join("lyrics").join(format!("{}.tex", item.id));
                    let lyrics_node = TexFile::new(lyrics_path);
                    let _ = g.add_root_node(lyrics_node);
                }
                _ => {}
            }
        }

        // Create matching PdfFile node (main.pdf in same directory)
        let pdf_path = parent_dir.join("main.pdf");
        let pdf_node = PdfFile::new(pdf_path.clone());
        let pdf_idx = match g.add_node(pdf_node) {
            Ok(idx) => idx,
            Err(_) => continue,
        };

        g.add_edge(song_idx, pdf_idx);
        built_songs.insert(
            song.info.file_stem_of_song(),
            (parent_dir.to_path_buf(), pdf_idx, song.info.clone()),
        );

        // Create CopyFile node to copy the PDF to ../pdf/<author>--@--<title>.pdf
        let pdf_copy_path =
            Path::new("../pdf").join(format!("{}.pdf", song.info.file_stem_of_song()));
        let pdf_copy_node = CopyFile::new(pdf_copy_path, "pdf".to_string());
        if let Ok(pdf_copy_idx) = g.add_node(pdf_copy_node) {
            g.add_edge(pdf_idx, pdf_copy_idx);
        }

        // Create lyrics PdfFile nodes (1-column and 2-column)
        for (filename, delivery_dir) in [
            ("main-1col.pdf", "pdf-lyrics-1-column"),
            ("main-2col.pdf", "pdf-lyrics-2-column"),
        ] {
            let lyrics_pdf_path = parent_dir.join("lyrics").join(filename);
            let lyrics_pdf_node = PdfFile::new(lyrics_pdf_path.clone());
            if let Ok(lyrics_pdf_idx) = g.add_node(lyrics_pdf_node) {
                g.add_edge(song_idx, lyrics_pdf_idx);

                let lyrics_copy_path = Path::new(&format!("../{delivery_dir}"))
                    .join(format!("{}-lyrics.pdf", song.info.file_stem_of_song()));
                let lyrics_copy_node = CopyFile::new(lyrics_copy_path, "pdf".to_string());
                if let Ok(lyrics_copy_idx) = g.add_node(lyrics_copy_node) {
                    g.add_edge(lyrics_pdf_idx, lyrics_copy_idx);
                }
            }
        }
    }

    // Every song of the world, whether or not the pattern kept it, so a book
    // listing a song that exists but was filtered out is told apart from a book
    // listing a song that does not exist at all.
    let known_songs: std::collections::HashSet<String> = world
        .items
        .iter()
        .filter_map(|(_, item)| match item {
            WorldItem::Song(song) => Some(song.info.file_stem_of_song()),
            WorldItem::Error(_) => None,
        })
        .collect();

    let books_srcdir = match books_srcdir {
        Some(dir) => dir,
        None => {
            if !world.books.is_empty() {
                log::error!("the world has books but no books source directory was given");
                return (false, g);
            }
            Path::new("")
        }
    };

    for (file_name, item) in &world.books {
        let book = match item {
            BookItem::Book(b) => b,
            BookItem::Error(_) => unreachable!(), // errors already checked above
        };
        let rel_path = Path::new(BOOKS_DIR).join(file_name);

        // Resolve the songs of the book. A song that is missing from the graph
        // because of the pattern makes the book unbuildable, but not the build:
        // building one song is a normal thing to ask for.
        let mut songs: Vec<BookSong> = Vec::new();
        let mut song_pdf_indices = Vec::new();
        let mut filtered_out = false;
        for stem in &book.songs {
            match built_songs.get(stem) {
                Some((dir, pdf_idx, info)) => {
                    songs.push(BookSong {
                        dir: dir.clone(),
                        author: info.author.clone(),
                        title: info.title.clone(),
                    });
                    song_pdf_indices.push(*pdf_idx);
                }
                None if known_songs.contains(stem) => {
                    log::info!(
                        "{}: song '{stem}' is filtered out by the pattern, skipping the book",
                        rel_path.display()
                    );
                    filtered_out = true;
                    break;
                }
                None => {
                    log::error!("{}: no song named '{stem}'", rel_path.display());
                    return (false, g);
                }
            }
        }
        if filtered_out {
            continue;
        }

        // The books are not under srcdir, so yamake cannot mount them: copy the
        // yaml into the sandbox, where the mount step picks it up as a file it
        // did not have to fetch, and digests it for change detection as usual.
        let sandbox_path = songs_sandbox.join(&rel_path);
        if let Some(parent) = sandbox_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::error!("Failed to create {}: {e}", parent.display());
            return (false, g);
        }
        if let Err(e) = std::fs::copy(books_srcdir.join(file_name), &sandbox_path) {
            log::error!(
                "Failed to copy {} to the sandbox: {e}",
                books_srcdir.join(file_name).display()
            );
            return (false, g);
        }

        let book_node = BookYml::new(rel_path.clone(), *book.clone(), songs);
        let pdf_path = book_node.pdf_path();
        let delivery_path = book_node.delivery_path();
        let book_idx = match g.add_root_node(book_node) {
            Ok(idx) => idx,
            Err(e) => {
                log::error!("{}: {e}", rel_path.display());
                return (false, g);
            }
        };

        // The book PDF is pre-added with Initial status, as for songs, and is
        // built after every song it collates: it imports their build directory.
        let pdf_idx = match g.add_node(PdfFile::new(pdf_path)) {
            Ok(idx) => idx,
            Err(e) => {
                log::error!("{}: {e}", rel_path.display());
                return (false, g);
            }
        };
        g.add_edge(book_idx, pdf_idx);
        for song_pdf_idx in song_pdf_indices {
            g.add_edge(song_pdf_idx, pdf_idx);
        }

        // Create CopyFile node to copy the PDF to ../pdf/book-<name>.pdf
        let copy_node = CopyFile::new(delivery_path, "pdf".to_string());
        if let Ok(copy_idx) = g.add_node(copy_node) {
            g.add_edge(pdf_idx, copy_idx);
        }
    }

    let success = g.make();

    // Copy make-report.yml from songs_sandbox to sandbox root
    // (keep the original so yamake can use it for incremental builds)
    let report_src = songs_sandbox.join("make-report.yml");
    let report_dest = sandbox.join("make-report.yml");
    if report_src.exists()
        && let Err(e) = std::fs::copy(&report_src, &report_dest)
    {
        log::error!("Failed to copy make-report.yml: {e}");
    }

    // Move logs directory from songs_sandbox to sandbox root
    let logs_src = songs_sandbox.join("logs");
    let logs_dest = sandbox.join("logs");
    if logs_dest.exists() {
        let _ = std::fs::remove_dir_all(&logs_dest);
    }
    if logs_src.exists()
        && let Err(e) = std::fs::rename(&logs_src, &logs_dest)
    {
        log::error!("Failed to move logs directory: {e}");
    }

    (success, g)
}

/// Async version of make_all that supports S3 paths for the source directories,
/// settings, and delivery.
///
/// This function:
/// - Downloads S3 songs and books source directories to temp directories if needed
/// - Downloads S3 settings to temp file if needed
/// - Calls existing `make_all()` with local paths
/// - Copies delivery files (pdf/, pdf-lyrics/) to the delivery path (local or S3)
/// - Cleans up temp directories
pub async fn make_all_with_storage(
    songs_srcdir: &str,
    books_srcdir: Option<&str>,
    sandbox: &Path,
    settings: Option<&str>,
    pattern: Option<&str>,
    delivery: &str,
    drum_patterns_dirs: &[PathBuf],
) -> Result<(bool, G), String> {
    use storage::{StoragePath, download_to_local};

    let srcdir = songs_srcdir;
    let srcdir_path = StoragePath::parse(srcdir)?;
    let settings_path = settings.map(StoragePath::parse).transpose()?;
    let delivery_path = StoragePath::parse(delivery)?;

    // Determine local paths for the build
    // This temp variable holds TempDir handle to keep directory alive until end of function
    let _temp_srcdir: Option<tempfile::TempDir>;

    let local_srcdir: std::path::PathBuf;

    // Handle srcdir
    if srcdir_path.is_s3() {
        let temp = tempfile::tempdir().map_err(|e| format!("Failed to create temp srcdir: {e}"))?;
        local_srcdir = temp.path().to_path_buf();
        _temp_srcdir = Some(temp);
        log::info!("Downloading srcdir from {srcdir} to {local_srcdir:?}");
        download_to_local(&srcdir_path, &local_srcdir).await?;
    } else {
        _temp_srcdir = None;
        local_srcdir = srcdir_path.as_local().unwrap().clone();
    }

    // Handle books srcdir, the same way as the songs srcdir
    let _temp_books: Option<tempfile::TempDir>;
    let local_books: Option<PathBuf>;
    match books_srcdir.map(StoragePath::parse).transpose()? {
        Some(books_path) if books_path.is_s3() => {
            let temp =
                tempfile::tempdir().map_err(|e| format!("Failed to create temp booksdir: {e}"))?;
            let local = temp.path().to_path_buf();
            _temp_books = Some(temp);
            log::info!("Downloading books from {books_srcdir:?} to {local:?}");
            download_to_local(&books_path, &local).await?;
            local_books = Some(local);
        }
        Some(books_path) => {
            _temp_books = None;
            local_books = Some(books_path.as_local().unwrap().clone());
        }
        None => {
            _temp_books = None;
            local_books = None;
        }
    }

    // Create local sandbox if it doesn't exist
    std::fs::create_dir_all(sandbox).map_err(|e| format!("Failed to create sandbox: {e}"))?;

    // Handle settings - download if S3, otherwise use local path
    let local_settings: Option<std::path::PathBuf>;
    let _temp_settings: Option<tempfile::NamedTempFile>;

    if let Some(ref sp) = settings_path {
        if sp.is_s3() {
            // Download settings file to a temp file
            let temp_file = tempfile::NamedTempFile::new()
                .map_err(|e| format!("Failed to create temp settings file: {e}"))?;
            let temp_path = temp_file.path().to_path_buf();

            // Download settings from S3
            let (bucket, prefix) = match sp {
                StoragePath::S3 { bucket, prefix } => (bucket, prefix),
                _ => unreachable!(),
            };
            let store = storage::create_s3_client(bucket).await?;

            let object_path = object_store::path::Path::from(prefix.as_str());
            let data = store
                .get(&object_path)
                .await
                .map_err(|e| format!("Failed to get settings from S3: {e}"))?;
            let bytes = data
                .bytes()
                .await
                .map_err(|e| format!("Failed to read settings from S3: {e}"))?;
            std::fs::write(&temp_path, &bytes)
                .map_err(|e| format!("Failed to write temp settings: {e}"))?;

            local_settings = Some(temp_path);
            _temp_settings = Some(temp_file);
        } else {
            local_settings = Some(sp.as_local().unwrap().clone());
            _temp_settings = None;
        }
    } else {
        local_settings = None;
        _temp_settings = None;
    }

    // Handle drum pattern directories - download from S3 if needed
    let mut _temp_drum_dirs: Vec<tempfile::TempDir> = Vec::new();
    let mut local_drum_dirs: Vec<PathBuf> = Vec::new();

    for dir in drum_patterns_dirs {
        let dir_str = dir.to_string_lossy();
        let dp = StoragePath::parse(&dir_str)?;
        if dp.is_s3() {
            let temp = tempfile::tempdir()
                .map_err(|e| format!("Failed to create temp drum patterns dir: {e}"))?;
            let local_path = temp.path().to_path_buf();
            log::info!("Downloading drum patterns from {dir_str} to {local_path:?}");
            download_to_local(&dp, &local_path).await?;
            local_drum_dirs.push(local_path);
            _temp_drum_dirs.push(temp);
        } else {
            local_drum_dirs.push(dir.clone());
        }
    }

    // Build the world and run the build
    let mut world = world_of_srcdir(&local_srcdir);
    if let Some(books) = &local_books {
        world.books = books_of_srcdir(books);
    }
    let (success, g) = make_all(
        &local_srcdir,
        local_books.as_deref(),
        sandbox,
        local_settings.as_deref(),
        pattern,
        &local_drum_dirs,
        &world,
    );

    // Helper function to collect files recursively
    fn collect_files_recursive(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() {
                    collect_files_recursive(&path, files);
                }
            }
        }
    }

    // Copy delivery files (pdf/ and pdf-lyrics/) to delivery path
    if delivery_path.is_s3() {
        let mut delivery_files: Vec<std::path::PathBuf> = Vec::new();

        let pdf_dir = sandbox.join("pdf");
        if pdf_dir.exists() {
            collect_files_recursive(&pdf_dir, &mut delivery_files);
        }

        for dir in [
            "pdf-lyrics-1-column",
            "pdf-lyrics-2-column",
            "tempo",
            "mp3",
            "mp3-renders",
            "mp3-with-clicks",
            "clicks",
            "pdf-snippets",
        ] {
            let sub_dir = sandbox.join(dir);
            if sub_dir.exists() {
                collect_files_recursive(&sub_dir, &mut delivery_files);
            }
        }

        log::info!(
            "Uploading {} delivery files to {delivery}",
            delivery_files.len()
        );
        storage::upload_paths_to_s3(&delivery_files, sandbox, &delivery_path).await?;
    } else {
        let local_delivery = delivery_path.as_local().unwrap();
        std::fs::create_dir_all(local_delivery)
            .map_err(|e| format!("Failed to create delivery directory: {e}"))?;

        for subdir in &[
            "pdf",
            "pdf-lyrics-1-column",
            "pdf-lyrics-2-column",
            "tempo",
            "mp3",
            "mp3-renders",
            "mp3-with-clicks",
            "clicks",
            "pdf-snippets",
        ] {
            let src_dir = sandbox.join(subdir);
            if !src_dir.exists() {
                continue;
            }
            let dest_dir = local_delivery.join(subdir);
            std::fs::create_dir_all(&dest_dir)
                .map_err(|e| format!("Failed to create delivery/{subdir}: {e}"))?;

            if let Ok(entries) = std::fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let dest = dest_dir.join(entry.file_name());
                        std::fs::copy(&path, &dest).map_err(|e| {
                            format!("Failed to copy {} to delivery: {e}", path.display())
                        })?;
                    }
                }
            }
        }
    }

    // Temp directories are automatically cleaned up when dropped
    Ok((success, g))
}

#[cfg(test)]
mod tests;
