use std::path::{Path, PathBuf};
use yamake::model::GNode;

use strudel_of_lilypond::sequencer::lilypond::strudel_of_sequence;

use crate::helpers::strudel_sequence_of_song;
use crate::model::Song;

/// Build node that generates an interactive Strudel HTML file from
/// the song's drum pattern sequence.
pub struct StrudelFile {
    /// Output path for `strudel.html` relative to the song directory.
    pub path: PathBuf,
    /// Song data used to derive the bar sequence.
    pub song: Song,
    /// Drum pattern library directories for resolving pattern names.
    pub libraries: Vec<PathBuf>,
}

impl StrudelFile {
    /// Creates a new `StrudelFile` node.
    pub fn new(path: PathBuf, song: Song, libraries: Vec<PathBuf>) -> Self {
        Self {
            path,
            song,
            libraries,
        }
    }
}

impl GNode for StrudelFile {
    fn tag(&self) -> String {
        "strudel".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn build(&self, sandbox: &Path, _predecessors: &[&(dyn GNode + Send + Sync)]) -> bool {
        let sequence = strudel_sequence_of_song(self.song.structure(), self.song.info.tempo);

        let dest = sandbox.join(&self.path);
        if let Some(parent) = dest.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::error!(
                "Failed to create directory for {}: {}",
                self.path.display(),
                e
            );
            return false;
        }

        if sequence.sequence.is_empty() {
            // Write empty HTML placeholder
            if let Err(e) = std::fs::write(&dest, "") {
                log::error!("Failed to write {}: {}", dest.display(), e);
                return false;
            }
            log::info!(
                "Empty sequence for {}, wrote empty file",
                self.path.display()
            );
            return true;
        }

        if self.libraries.is_empty() {
            log::info!("No drum pattern libraries provided, skipping strudel generation");
            if let Err(e) = std::fs::write(&dest, "") {
                log::error!("Failed to write {}: {}", dest.display(), e);
                return false;
            }
            return true;
        }

        let title = &self.song.info.title;
        let author = &self.song.info.author;
        let html = match strudel_of_sequence(&sequence, &self.libraries, title) {
            Ok(h) => {
                let header = format!("// author : {author}\n// title : {title}\n");
                h.replacen("<!--\n", &format!("<!--\n{header}"), 1)
            }
            Err(e) => {
                log::error!(
                    "Failed to generate strudel for {}: {}",
                    self.path.display(),
                    e
                );
                return false;
            }
        };

        if let Err(e) = std::fs::write(&dest, &html) {
            log::error!("Failed to write {}: {}", dest.display(), e);
            return false;
        }

        log::info!("Wrote strudel HTML to {}", dest.display());
        true
    }
}
