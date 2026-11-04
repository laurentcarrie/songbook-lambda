use std::path::{Path, PathBuf};
use yamake::model::GNode;

/// Build node for a `.lytex` file (LilyPond embedded in LaTeX).
pub struct LyTexFile {
    pub path: PathBuf,
}

impl LyTexFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl GNode for LyTexFile {
    fn tag(&self) -> String {
        "lytex".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn build(&self, sandbox: &Path, predecessors: &[&(dyn GNode + Send + Sync)]) -> bool {
        // Find predecessors of type "lilypond"
        // Several lilypond predecessors are legitimate: the score itself plus
        // every file it \include-s, which are wired in so that editing a
        // definitions file invalidates this node. Pick the one whose stem
        // matches our own - that is the score this output is built from.
        let want = self.path.file_stem().and_then(|s| s.to_str());
        let ly_predecessors: Vec<_> = predecessors
            .iter()
            .filter(|p| p.tag() == "lilypond")
            .filter(|p| p.pathbuf().file_stem().and_then(|s| s.to_str()) == want)
            .collect();

        if ly_predecessors.len() != 1 {
            log::error!(
                "{} {} expects one lilypond predecessor named {}, found {}",
                "LyTexFile",
                self.path.display(),
                want.unwrap_or("?"),
                ly_predecessors.len()
            );
            return false;
        }

        let ly_file = ly_predecessors[0];
        let ly_pathbuf = ly_file.pathbuf();
        let ly_filename = ly_pathbuf
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.ly");

        let full_path = sandbox.join(&self.path);

        // Create parent directory if needed
        if let Some(parent) = full_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::error!(
                "Failed to create directory for {}: {}",
                self.path.display(),
                e
            );
            return false;
        }

        // Fingerprint of the score and every file it \include-s, as a LaTeX
        // comment. The build keys downstream nodes on this file's content, and
        // `\lilypondfile{x.ly}` alone never changes when only an included
        // definitions file does - so lilypond-book would not be rerun.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for p in predecessors.iter().filter(|p| p.tag() == "lilypond") {
            let path = p.pathbuf();
            std::hash::Hash::hash(&path, &mut hasher);
            if let Ok(bytes) = std::fs::read(sandbox.join(&path)) {
                std::hash::Hash::hash(&bytes, &mut hasher);
            }
        }
        let fingerprint = std::hash::Hasher::finish(&hasher);

        let content = format!("\\lilypondfile{{{ly_filename}}}\n% sources: {fingerprint:016x}\n");

        if let Err(e) = std::fs::write(&full_path, &content) {
            log::error!("Failed to write {}: {}", self.path.display(), e);
            return false;
        }

        true
    }
}
