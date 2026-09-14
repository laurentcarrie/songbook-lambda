use handlebars::Handlebars;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use yamake::model::{Edge, ExpandError, ExpandResult, GNode, GRootNode};

use super::{
    ClickYml, CopyFile, LilypondFile, LyTexFile, MidiOfLilypond, Mp3, Mp3Render, PdfFile,
    PdfOfLilypond, SongTikz, StrudelFile, TexFile, TexOfLilypond,
};
use crate::chords::bar_numbering::barcount_map_of_structure;
use crate::helpers::register_helpers;
use crate::model::{SectionItem, Song};
use crate::settings::Settings;

const SONG_TIKZ_TEMPLATE: &str = include_str!("../resources/texfiles/song.tikz");
const PREAMBLE_TEMPLATE: &str = include_str!("../resources/texfiles/preamble.tex");
const TIKZ_SPLINE_LIB: &str = include_str!("../resources/texfiles/tikzlibraryspline.code.tex");
const SECTIONS_TEMPLATE: &str = include_str!("../resources/texfiles/sections.tex");
const CHORDS_TEX: &str = include_str!("../resources/texfiles/chords.tex");
const DATA_TEMPLATE: &str = include_str!("../resources/texfiles/data.tex");
const MACROS_LY_TEMPLATE: &str = include_str!("../resources/lyfiles/macros.ly");
const MAIN_LYRICS_1COL_TEMPLATE: &str = include_str!("../resources/texfiles/main-lyrics-1col.tex");
const MAIN_LYRICS_2COL_TEMPLATE: &str = include_str!("../resources/texfiles/main-lyrics-2col.tex");

/// Root node for `song.yml` that expands into the full build graph
/// (TeX files, PDF, lyrics, LilyPond, and Strudel nodes).
pub struct SongYml {
    /// Path to the `song.yml` file relative to srcdir.
    pub path: PathBuf,
    /// Parsed song data.
    pub song: Song,
    /// Directories containing drum pattern library files.
    pub drum_patterns_dirs: Vec<PathBuf>,
    /// Root directory where source files live. Empty when unset, in which case
    /// declared source files are looked up in the sandbox instead.
    pub srcdir: PathBuf,
}

impl SongYml {
    /// Creates a new `SongYml` node with the given path and song data.
    pub fn new(path: PathBuf, song: Song) -> Self {
        Self {
            path,
            song,
            drum_patterns_dirs: vec![],
            srcdir: PathBuf::new(),
        }
    }

    /// Sets the drum pattern library directories (builder pattern).
    pub fn with_drum_patterns_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.drum_patterns_dirs = dirs;
        self
    }

    /// Sets the source directory (builder pattern).
    pub fn with_srcdir(mut self, srcdir: &Path) -> Self {
        self.srcdir = srcdir.to_path_buf();
        self
    }

    /// Whether a file declared in `song.yml` exists. Looks in srcdir when it is
    /// known, and in the sandbox otherwise (the source has not been mounted yet
    /// the first time a newly declared file is seen).
    /// Reads a source file the same way [`Self::source_exists`] locates it.
    fn read_source(&self, sandbox: &Path, rel: &Path) -> Option<String> {
        let base = if self.srcdir.as_os_str().is_empty() {
            sandbox
        } else {
            self.srcdir.as_path()
        };
        std::fs::read_to_string(base.join(rel)).ok()
    }

    /// Collects the `\include "..."` targets of `roots`, following them
    /// recursively. Reads the SOURCES, not the sandbox copies: on a clean build
    /// the sandbox is still empty when the graph is built, so scanning it there
    /// finds nothing and the first build fails before converging on the second.
    fn included_ly_of(&self, sandbox: &Path, roots: &[PathBuf]) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = vec![];
        let mut seen: Vec<PathBuf> = vec![];
        let mut queue: Vec<PathBuf> = roots.to_vec();
        while let Some(cur) = queue.pop() {
            if seen.contains(&cur) {
                continue;
            }
            seen.push(cur.clone());
            let Some(content) = self.read_source(sandbox, &cur) else {
                continue;
            };
            let parent = cur.parent().unwrap_or(Path::new("")).to_path_buf();
            for line in content.lines() {
                if line.trim_start().starts_with('%') {
                    continue;
                }
                let Some(pos) = line.find("\\include") else {
                    continue;
                };
                let rest = &line[pos + 8..];
                let Some(q1) = rest.find('"') else { continue };
                let after = &rest[q1 + 1..];
                let Some(q2) = after.find('"') else { continue };
                let target = parent.join(&after[..q2]);
                if !found.contains(&target) {
                    found.push(target.clone());
                }
                queue.push(target);
            }
        }
        found
    }

    fn source_exists(&self, sandbox: &Path, rel: &Path) -> bool {
        if self.srcdir.as_os_str().is_empty() {
            sandbox.join(rel).is_file()
        } else {
            self.srcdir.join(rel).is_file()
        }
    }
}

impl GRootNode for SongYml {
    fn tag(&self) -> String {
        "song.yml".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn expand(&self, sandbox: &Path, _predecessors: &[&(dyn GNode + Send + Sync)]) -> ExpandResult {
        // Get the directory containing song.yml
        let parent_dir = self.path.parent().unwrap_or(Path::new(""));

        let mut song = self.song.clone();

        // Load settings from settings.yml at sandbox root
        let settings = Settings::load(sandbox).map_err(ExpandError::Other)?;
        let section_colors = &settings.colors;

        // Resolve colors for Chords items using color_of_section
        for item in &mut song.structure {
            if let SectionItem::Chords(chords) = &mut item.item {
                let color = section_colors
                    .color_of_section(&chords.section_type, chords.color.as_deref())
                    .map_err(ExpandError::Other)?;
                chords.color = Some(color);
            }
        }

        // Build a map of Chords id -> color for Ref color resolution
        let chords_colors: std::collections::HashMap<String, String> = song
            .structure
            .iter()
            .filter_map(|item| match &item.item {
                SectionItem::Chords(chords) => {
                    Some((item.id.clone(), chords.color.clone().unwrap_or_default()))
                }
                _ => None,
            })
            .collect();

        // Validate Ref items and fill in missing colors from linked Chords
        for item in &mut song.structure {
            if let SectionItem::Ref(ref_section) = &mut item.item {
                if let Some(linked_color) = chords_colors.get(&ref_section.link) {
                    // Fill in color from linked Chords if not specified
                    if ref_section.color.is_none() {
                        ref_section.color = Some(linked_color.clone());
                    }
                } else {
                    let error_msg = format!(
                        "Ref '{}' links to '{}' but no Chords item with that id exists",
                        item.id, ref_section.link
                    );
                    log::error!("{error_msg}");
                    return Err(ExpandError::Other(error_msg));
                }
            }
        }

        // Convert Song to JSON for handlebars templates
        let song_data = match serde_json::to_value(&song) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to serialize song data: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        // Extract unique section types for sections.tex
        let mut section_types: HashSet<String> = HashSet::new();
        for item in &song.structure {
            match &item.item {
                SectionItem::Chords(chords) => {
                    section_types.insert(chords.section_type.clone());
                }
                SectionItem::Ref(ref_section) => {
                    if let Some(ref st) = ref_section.section_type {
                        section_types.insert(st.clone());
                    }
                }
                _ => {}
            }
        }

        // Build sections data with default colors
        let sections: Vec<serde_json::Value> = section_types
            .into_iter()
            .map(|id| {
                let color = match id.as_str() {
                    "intro" => "yellow!20",
                    "couplet" => "green!20",
                    "coupletb" => "green!10",
                    "refrain" => "blue!20",
                    "pont" => "orange!20",
                    "outro" => "red!20",
                    _ => "gray!20",
                };
                serde_json::json!({"id": id, "color": color})
            })
            .collect();

        // Create main.tex path relative to song.yml location
        let tex_path = parent_dir.join("main.tex");
        let pdf_path = parent_dir.join("main.pdf");

        // Create the main.tex file in the sandbox
        let tex_full_path = sandbox.join(&tex_path);
        if let Some(parent) = tex_full_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tex_content = r#"\documentclass{article}
\PassOptionsToPackage{x11names}{xcolor}
\usepackage{tikz}
\input{preamble}
\input{chords}
\input{sections}
\input{data}
\begin{document}
\input{body}
\end{document}
"#
        .to_string();
        if let Err(e) = std::fs::write(&tex_full_path, &tex_content) {
            log::error!("Failed to write main.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create song.tikz file using handlebars template
        let tikz_path = parent_dir.join("song.tikz");
        let tikz_full_path = sandbox.join(&tikz_path);

        // Render the templates
        let mut handlebars = Handlebars::new();
        register_helpers(&mut handlebars);
        // First and last bar of every Chords/Ref section, by section id
        let bar_map = barcount_map_of_structure(&song.structure);
        let bar_range_of = |id: &str| {
            bar_map
                .get(id)
                .map(|(rows, end)| (rows.first().copied().unwrap_or(1), end - 1))
                .unwrap_or((1, 1))
        };
        let bar_ranges: serde_json::Map<String, serde_json::Value> = song
            .structure
            .iter()
            .map(|item| {
                let (first, last) = bar_range_of(&item.id);
                (item.id.clone(), serde_json::json!({"first": first, "last": last}))
            })
            .collect();
        let template_data = serde_json::json!({"song": song_data, "sections": sections, "settings": settings, "bar_ranges": bar_ranges});

        // Render song.tikz
        let tikz_content = match handlebars.render_template(SONG_TIKZ_TEMPLATE, &template_data) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to render song.tikz template: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        if let Err(e) = std::fs::write(&tikz_full_path, &tikz_content) {
            log::error!("Failed to write song.tikz: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create preamble.tex file using handlebars template
        let preamble_path = parent_dir.join("preamble.tex");
        let preamble_full_path = sandbox.join(&preamble_path);
        let preamble_content = match handlebars.render_template(PREAMBLE_TEMPLATE, &template_data) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to render preamble.tex template: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        if let Err(e) = std::fs::write(&preamble_full_path, &preamble_content) {
            log::error!("Failed to write preamble.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create tikzlibraryspline.code.tex file
        let spline_path = parent_dir.join("tikzlibraryspline.code.tex");
        let spline_full_path = sandbox.join(&spline_path);
        if let Err(e) = std::fs::write(&spline_full_path, TIKZ_SPLINE_LIB) {
            log::error!("Failed to write tikzlibraryspline.code.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create sections.tex file using handlebars template
        let sections_path = parent_dir.join("sections.tex");
        let sections_full_path = sandbox.join(&sections_path);
        let sections_content = match handlebars.render_template(SECTIONS_TEMPLATE, &template_data) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to render sections.tex template: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        if let Err(e) = std::fs::write(&sections_full_path, &sections_content) {
            log::error!("Failed to write sections.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create chords.tex file
        let chords_path = parent_dir.join("chords.tex");
        let chords_full_path = sandbox.join(&chords_path);
        if let Err(e) = std::fs::write(&chords_full_path, CHORDS_TEX) {
            log::error!("Failed to write chords.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create data.tex file using handlebars template
        let data_path = parent_dir.join("data.tex");
        let data_full_path = sandbox.join(&data_path);
        let data_content = match handlebars.render_template(DATA_TEMPLATE, &template_data) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to render data.tex template: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        if let Err(e) = std::fs::write(&data_full_path, &data_content) {
            log::error!("Failed to write data.tex: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create macros.ly file using handlebars template
        let macros_ly_path = parent_dir.join("macros.ly");
        let macros_ly_full_path = sandbox.join(&macros_ly_path);
        let macros_ly_content = match handlebars.render_template(MACROS_LY_TEMPLATE, &template_data)
        {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to render macros.ly template: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        };

        if let Err(e) = std::fs::write(&macros_ly_full_path, &macros_ly_content) {
            log::error!("Failed to write macros.ly: {e}");
            return Err(ExpandError::Other(e.to_string()));
        }

        // Create lyrics tex files with inputs for each Chords/Ref section
        let lyrics_dir_full = sandbox.join(parent_dir.join("lyrics"));
        let _ = std::fs::create_dir_all(&lyrics_dir_full);

        // Build the lyrics_inputs content (shared by both 1col and 2col)
        let mut lyrics_inputs = String::new();
        for item in &song.structure {
            match &item.item {
                SectionItem::Chords(chords) => {
                    let color = chords.color.as_deref().unwrap_or("white");
                    let (first_bar, last_bar) = bar_range_of(&item.id);
                    lyrics_inputs.push_str(&format!(
                        "\\basecouplet{{{}}}{{ \\hfill {} \\hfill \\llap{{\\fontsize{{12pt}}{{12pt}}\\selectfont ({} $\\rightarrow$ {})}} }}{{\\input{{{}}}}}\n\n",
                        color, chords.title, first_bar, last_bar, item.id
                    ));
                }
                SectionItem::Ref(ref_section) => {
                    let color = ref_section.color.as_deref().unwrap_or("white");
                    let (first_bar, last_bar) = bar_range_of(&item.id);
                    lyrics_inputs.push_str(&format!(
                        "\\basecouplet{{{}}}{{ \\hfill {} \\hfill \\llap{{\\fontsize{{12pt}}{{12pt}}\\selectfont ({} $\\rightarrow$ {})}} }}{{\\input{{{}}}}}\n\n",
                        color, ref_section.title, first_bar, last_bar, item.id
                    ));
                }
                _ => {}
            }
        }

        let lyrics_template_data = serde_json::json!({"song": song_data, "lyrics_inputs": lyrics_inputs, "settings": settings});

        // Render and write both 1-column and 2-column lyrics tex files
        for (template, filename) in [
            (MAIN_LYRICS_1COL_TEMPLATE, "main-1col.tex"),
            (MAIN_LYRICS_2COL_TEMPLATE, "main-2col.tex"),
        ] {
            let tex_content = match handlebars.render_template(template, &lyrics_template_data) {
                Ok(content) => content,
                Err(e) => {
                    log::error!("Failed to render {filename} template: {e}");
                    return Err(ExpandError::Other(e.to_string()));
                }
            };

            let tex_full_path = lyrics_dir_full.join(filename);
            if let Err(e) = std::fs::write(&tex_full_path, &tex_content) {
                log::error!("Failed to write lyrics/{filename}: {e}");
                return Err(ExpandError::Other(e.to_string()));
            }
        }

        // Create nodes (body.tex and lyrics files are added as root nodes in make_all)
        let tex_node = TexFile::new(tex_path.clone());
        let tikz_node = SongTikz::new(tikz_path.clone());
        let preamble_node = TexFile::new(preamble_path);
        let spline_node = TexFile::new(spline_path);
        let sections_node = TexFile::new(sections_path);
        let chords_node = TexFile::new(chords_path);
        let data_node = TexFile::new(data_path);
        let macros_ly_node = LilypondFile::new(macros_ly_path);

        // Lyrics nodes: 1-column and 2-column variants
        let lyrics_1col_tex_path = parent_dir.join("lyrics").join("main-1col.tex");
        let lyrics_2col_tex_path = parent_dir.join("lyrics").join("main-2col.tex");
        let lyrics_1col_tex_node = TexFile::new(lyrics_1col_tex_path.clone());
        let lyrics_2col_tex_node = TexFile::new(lyrics_2col_tex_path.clone());

        // Scan for lilypond files referenced in tex files
        let pdf_for_scan = PdfFile::new(pdf_path.clone());
        let predecessors: Vec<&(dyn GNode + Send + Sync)> = vec![&tex_node];
        let (_, scanned_inputs, toplevel_ly) =
            pdf_for_scan.scan_with_toplevel_ly(sandbox, &predecessors);

        // Only use top-level .ly files (from \lyfile{} or \songly{}) for LyTexFile nodes
        // Included .ly files (from \include) don't need LyTexFile/TexOfLilypond
        let mut ly_files = toplevel_ly;

        // Files listed under `files.lilypond` in song.yml are dependencies too.
        // The scan above only sees .ly reachable from the tex sources, so a file
        // the song declares explicitly would otherwise never rebuild when edited.
        // A declared file that is not on disk is skipped with a warning rather
        // than failing the build: the field was ignored for a long time, so
        // song.yml files in the wild still carry stale names.
        for declared in &self.song.files.lilypond {
            let name = if declared.ends_with(".ly") {
                declared.clone()
            } else {
                format!("{declared}.ly")
            };
            let ly_path = parent_dir.join(name);
            if !self.source_exists(sandbox, &ly_path) {
                log::warn!(
                    "{}: files.lilypond lists '{declared}' but {} does not exist - ignoring",
                    self.path.display(),
                    ly_path.display()
                );
                continue;
            }
            if !ly_files.contains(&ly_path) {
                ly_files.push(ly_path);
            }
        }

        // .ly files reached only through `\include`. They must be mounted into the
        // sandbox, or lilypond cannot resolve the include and the build dies with
        // "unknown escaped string" on whatever the included file defines. They get
        // a LilypondFile node and nothing else: unlike ly_files they carry no
        // \score of their own, so giving them the lytex/PDF chain would fail with
        // "lilypond produced no cropped PDF".
        // Two lists, because the two uses do not overlap:
        //   included_ly  - files to MOUNT, so only those that exist in the
        //                  sources; a generated one is already in the sandbox.
        //   included_dep - files to hang DEPENDENCY EDGES on, which must also
        //                  cover generated ones. macros.ly is the case that
        //                  matters: it carries songtempo from song.yml, so
        //                  editing the tempo has to invalidate the .midi and
        //                  the .mp3 rendered from it.
        let mut included_ly: Vec<PathBuf> = vec![];
        let mut included_dep: Vec<PathBuf> = vec![];
        let from_sources = self.included_ly_of(sandbox, &ly_files);
        for input in scanned_inputs.iter().chain(from_sources.iter()) {
            if !input.extension().map(|e| e == "ly").unwrap_or(false) || ly_files.contains(input) {
                continue;
            }
            let in_source = self.source_exists(sandbox, input);
            if in_source && !included_ly.contains(input) {
                included_ly.push(input.clone());
            }
            if (in_source || sandbox.join(input).is_file()) && !included_dep.contains(input) {
                included_dep.push(input.clone());
            }
        }

        // Audio renders declared under `files.mp3`: <stem>.mp3 is synthesised
        // from <stem>.ly via LilyPond's MIDI output. Declaring a render does not
        // put the score in the PDF - only files.lilypond and \songly{} do that,
        // so remember whether this .ly already has a LilypondFile node.
        let mut render_targets: Vec<(PathBuf, PathBuf, PathBuf, bool)> = vec![];
        for declared in &self.song.files.mp3 {
            let stem = declared.strip_suffix(".mp3").unwrap_or(declared);
            let ly_path = parent_dir.join(format!("{stem}.ly"));
            if !self.source_exists(sandbox, &ly_path) {
                log::warn!(
                    "{}: files.mp3 lists '{declared}' but {} does not exist - ignoring",
                    self.path.display(),
                    ly_path.display()
                );
                continue;
            }
            let already_mounted = ly_files.contains(&ly_path);
            render_targets.push((
                ly_path,
                parent_dir.join(format!("{stem}.midi")),
                parent_dir.join(format!("{stem}.mp3")),
                already_mounted,
            ));
        }

        // Create lyrics PDF paths (1-column and 2-column)
        let lyrics_1col_pdf_path = parent_dir.join("lyrics").join("main-1col.pdf");
        let lyrics_2col_pdf_path = parent_dir.join("lyrics").join("main-2col.pdf");
        let lyrics_1col_pdf_node = PdfFile::new(lyrics_1col_pdf_path.clone());
        let lyrics_2col_pdf_node = PdfFile::new(lyrics_2col_pdf_path.clone());

        let mut nodes: Vec<Box<dyn GNode + Send + Sync>> = vec![
            Box::new(tex_node),
            Box::new(tikz_node),
            Box::new(preamble_node),
            Box::new(spline_node),
            Box::new(sections_node),
            Box::new(chords_node),
            Box::new(data_node),
            Box::new(macros_ly_node),
            Box::new(lyrics_1col_tex_node),
            Box::new(lyrics_2col_tex_node),
            Box::new(lyrics_1col_pdf_node),
            Box::new(lyrics_2col_pdf_node),
        ];

        // Mount the \include-only .ly files (see above): source nodes, no chain.
        for inc in &included_ly {
            nodes.push(Box::new(LilypondFile::new(inc.clone())));
        }

        let mut edges: Vec<Edge> = vec![];

        // NOTE: PdfFile must be pre-added to the graph with Initial status
        // before calling make(), otherwise yamake marks expanded nodes as
        // "mounted" which skips the build phase
        let main_edge = Edge {
            nfrom: Box::new(TexFile::new(tex_path)),
            nto: Box::new(PdfFile::new(pdf_path.clone())),
        };
        edges.push(main_edge);

        // Edge: lyrics/main-1col.tex -> lyrics/main-1col.pdf
        let lyrics_1col_edge = Edge {
            nfrom: Box::new(TexFile::new(lyrics_1col_tex_path)),
            nto: Box::new(PdfFile::new(lyrics_1col_pdf_path)),
        };
        edges.push(lyrics_1col_edge);

        // Edge: lyrics/main-2col.tex -> lyrics/main-2col.pdf
        let lyrics_2col_edge = Edge {
            nfrom: Box::new(TexFile::new(lyrics_2col_tex_path)),
            nto: Box::new(PdfFile::new(lyrics_2col_pdf_path)),
        };
        edges.push(lyrics_2col_edge);

        // Kept for the snippet-PDF chain below, which needs the same list after
        // this loop has consumed it.
        let snippet_sources = ly_files.clone();

        // Add LilypondFile -> LyTexFile -> PdfFile chain
        // Add LilypondFile -> TexOfLilypond -> PdfFile chain
        for ly_path in ly_files {
            // Get stem (e.g., "interlude" from "parent/interlude.ly")
            let stem = ly_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let ly_parent = ly_path.parent().unwrap_or(Path::new(""));

            // Create LyTexFile path (same as ly but with .lytex extension)
            let lytex_path = ly_path.with_extension("lytex");

            // Create TexOfLilypond path (e.g., parent/interlude.output/interlude.tex)
            let texoflypath = ly_parent
                .join(format!("{stem}.output"))
                .join(format!("{stem}.tex"));

            let ly_node = LilypondFile::new(ly_path.clone());
            let lytex_node = LyTexFile::new(lytex_path.clone());
            let texofly_node = TexOfLilypond::new(texoflypath.clone());
            nodes.push(Box::new(ly_node));
            nodes.push(Box::new(lytex_node));
            nodes.push(Box::new(texofly_node));

            // Same reasoning as for MidiOfLilypond below: an included file has
            // to invalidate the snippet chain of the score that includes it.
            for inc in &included_dep {
                edges.push(Edge {
                    nfrom: Box::new(LilypondFile::new(inc.clone())),
                    nto: Box::new(LyTexFile::new(lytex_path.clone())),
                });
            }

            // Edge: LilypondFile -> LyTexFile
            let ly_to_lytex_edge = Edge {
                nfrom: Box::new(LilypondFile::new(ly_path.clone())),
                nto: Box::new(LyTexFile::new(lytex_path.clone())),
            };
            edges.push(ly_to_lytex_edge);

            // Edge: LyTexFile -> PdfFile
            let lytex_to_pdf_edge = Edge {
                nfrom: Box::new(LyTexFile::new(lytex_path.clone())),
                nto: Box::new(PdfFile::new(pdf_path.clone())),
            };
            edges.push(lytex_to_pdf_edge);

            // Edge: LilypondFile -> TexOfLilypond
            let ly_to_texofly_edge = Edge {
                nfrom: Box::new(LilypondFile::new(ly_path)),
                nto: Box::new(TexOfLilypond::new(texoflypath.clone())),
            };
            edges.push(ly_to_texofly_edge);

            // Edge: LyTexFile -> TexOfLilypond
            let lytex_to_texofly_edge = Edge {
                nfrom: Box::new(LyTexFile::new(lytex_path)),
                nto: Box::new(TexOfLilypond::new(texoflypath.clone())),
            };
            edges.push(lytex_to_texofly_edge);

            // Edge: TexOfLilypond -> PdfFile
            let texofly_to_pdf_edge = Edge {
                nfrom: Box::new(TexOfLilypond::new(texoflypath)),
                nto: Box::new(PdfFile::new(pdf_path.clone())),
            };
            edges.push(texofly_to_pdf_edge);
        }

        // Pre-compute file stem for delivery copy nodes (before song is moved)
        let file_stem = song.info.file_stem_of_song();

        // Add LilypondFile -> PdfOfLilypond chain: one standalone cropped PDF per
        // LilyPond file, delivered so each score can be shown on its own.
        for ly_path in snippet_sources {
            let section = ly_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let snippet_pdf_path = ly_path.with_extension("pdf");
            nodes.push(Box::new(PdfOfLilypond::new(snippet_pdf_path.clone())));

            // Edge: LilypondFile -> PdfOfLilypond
            edges.push(Edge {
                nfrom: Box::new(LilypondFile::new(ly_path)),
                nto: Box::new(PdfOfLilypond::new(snippet_pdf_path.clone())),
            });

            // Copy to ../pdf-snippets/<name>-<section>.pdf
            let snippet_copy_path =
                Path::new("../pdf-snippets").join(format!("{file_stem}-{section}.pdf"));
            nodes.push(Box::new(CopyFile::new(
                snippet_copy_path.clone(),
                "pdfsnippet".to_string(),
            )));

            // Edge: PdfOfLilypond -> pdf-snippets/<name>-<section>.pdf
            edges.push(Edge {
                nfrom: Box::new(PdfOfLilypond::new(snippet_pdf_path)),
                nto: Box::new(CopyFile::new(snippet_copy_path, "pdfsnippet".to_string())),
            });
        }

        // Add LilypondFile -> MidiOfLilypond -> Mp3Render chain for files.mp3
        for (ly_path, midi_path, mp3_render_path, already_mounted) in render_targets {
            if !already_mounted {
                nodes.push(Box::new(LilypondFile::new(ly_path.clone())));
            }
            nodes.push(Box::new(MidiOfLilypond::new(midi_path.clone())));
            nodes.push(Box::new(Mp3Render::new(mp3_render_path.clone())));

            // Edge: LilypondFile -> MidiOfLilypond
            edges.push(Edge {
                nfrom: Box::new(LilypondFile::new(ly_path)),
                nto: Box::new(MidiOfLilypond::new(midi_path.clone())),
            });

            // Edges: every \include-d .ly -> MidiOfLilypond. Without these a
            // change to a definitions file leaves the .midi - and the .mp3
            // rendered from it - stale: MidiOfLilypond declares no scan() of
            // its own, so the graph only knows about the score file itself.
            for inc in &included_dep {
                edges.push(Edge {
                    nfrom: Box::new(LilypondFile::new(inc.clone())),
                    nto: Box::new(MidiOfLilypond::new(midi_path.clone())),
                });
            }

            // Edge: MidiOfLilypond -> Mp3Render
            edges.push(Edge {
                nfrom: Box::new(MidiOfLilypond::new(midi_path)),
                nto: Box::new(Mp3Render::new(mp3_render_path.clone())),
            });

            // Copy the render to ../mp3-renders/<name>-<section>.mp3. A song can
            // have several, so the section name is part of the delivered name.
            let section = mp3_render_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let render_copy_path =
                Path::new("../mp3-renders").join(format!("{file_stem}-{section}.mp3"));
            nodes.push(Box::new(CopyFile::new(
                render_copy_path.clone(),
                "mp3render".to_string(),
            )));

            // Edge: Mp3Render -> mp3-renders/<name>-<section>.mp3
            edges.push(Edge {
                nfrom: Box::new(Mp3Render::new(mp3_render_path)),
                nto: Box::new(CopyFile::new(render_copy_path, "mp3render".to_string())),
            });
        }

        // Create StrudelFile node
        let strudel_path = parent_dir.join("strudel.html");
        let libraries = self.drum_patterns_dirs.clone();
        let strudel_node = StrudelFile::new(strudel_path.clone(), song.clone(), libraries.clone());
        nodes.push(Box::new(strudel_node));

        // Edge: song.yml -> strudel.html
        edges.push(Edge {
            nfrom: Box::new(SongYml::new(self.path.clone(), self.song.clone())),
            nto: Box::new(StrudelFile::new(
                strudel_path.clone(),
                song.clone(),
                libraries.clone(),
            )),
        });

        // Create CopyFile node to copy strudel.html to ../tempo/<author>--@--<title>.html
        let strudel_copy_path = Path::new("../tempo").join(format!("{}.html", file_stem));
        let strudel_copy_node = CopyFile::new(strudel_copy_path.clone(), "strudel".to_string());
        nodes.push(Box::new(strudel_copy_node));

        // Edge: strudel.html -> tempo/<name>.html
        edges.push(Edge {
            nfrom: Box::new(StrudelFile::new(strudel_path, song, libraries)),
            nto: Box::new(CopyFile::new(strudel_copy_path, "strudel".to_string())),
        });

        // Mount click-related nodes if has_clicks is true
        if self.song.files.has_clicks && !self.song.files.has_mp3 {
            return Err(ExpandError::Other(format!(
                "{}: has_clicks is true but has_mp3 is false — song.mp3 is required for click overlay",
                self.path.display()
            )));
        }
        if self.song.files.has_clicks {
            let clicks_yml_path = parent_dir.join("clicks.yml");
            nodes.push(Box::new(ClickYml::new(clicks_yml_path.clone())));

            // TODO: re-enable song-with-click.mp3 generation
            // let check_mp3_path = parent_dir.join("song-with-click.mp3");
            // nodes.push(Box::new(ClickCheckMp3::new(check_mp3_path.clone())));
            // edges.push(Edge {
            //     nfrom: Box::new(ClickYml::new(clicks_yml_path.clone())),
            //     nto: Box::new(ClickCheckMp3::new(check_mp3_path.clone())),
            // });
            // let song_mp3_path = parent_dir.join("song.mp3");
            // edges.push(Edge {
            //     nfrom: Box::new(Mp3::new(song_mp3_path)),
            //     nto: Box::new(ClickCheckMp3::new(check_mp3_path.clone())),
            // });
            // let click_mp3_copy_path =
            //     Path::new("../mp3-with-clicks").join(format!("{}-with-clicks.mp3", file_stem));
            // let click_mp3_copy_node =
            //     CopyFile::new(click_mp3_copy_path.clone(), "click_check_mp3".to_string());
            // nodes.push(Box::new(click_mp3_copy_node));
            // edges.push(Edge {
            //     nfrom: Box::new(ClickCheckMp3::new(check_mp3_path)),
            //     nto: Box::new(CopyFile::new(
            //         click_mp3_copy_path,
            //         "click_check_mp3".to_string(),
            //     )),
            // });

            // Copy clicks.yml to ../clicks/<name>-clicks.yml
            let clicks_copy_path = Path::new("../clicks").join(format!("{}-clicks.yml", file_stem));
            let clicks_copy_node =
                CopyFile::new(clicks_copy_path.clone(), "clicks.yml".to_string());
            nodes.push(Box::new(clicks_copy_node));

            edges.push(Edge {
                nfrom: Box::new(ClickYml::new(clicks_yml_path)),
                nto: Box::new(CopyFile::new(clicks_copy_path, "clicks.yml".to_string())),
            });
        }

        // Mount song.mp3 if has_mp3 is true
        if self.song.files.has_mp3 {
            let song_mp3_path = parent_dir.join("song.mp3");
            nodes.push(Box::new(Mp3::new(song_mp3_path.clone())));

            // Copy song.mp3 to ../mp3/<name>.mp3
            let mp3_copy_path = Path::new("../mp3").join(format!("{}.mp3", file_stem));
            let mp3_copy_node = CopyFile::new(mp3_copy_path.clone(), "mp3".to_string());
            nodes.push(Box::new(mp3_copy_node));

            edges.push(Edge {
                nfrom: Box::new(Mp3::new(song_mp3_path)),
                nto: Box::new(CopyFile::new(mp3_copy_path, "mp3".to_string())),
            });
        }

        Ok((nodes, edges))
    }
}
