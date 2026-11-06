use std::path::{Path, PathBuf};
use yamake::model::GNode;

/// Name of the file `lualatex` writes next to a song's `main.pdf`.
pub const CELLS_FILE: &str = "cells.yml";

/// Build node that delivers the page position of every bar of a song PDF.
///
/// The positions are a by-product of the `lualatex` run that builds the PDF
/// (see `\songplacechart` in `preamble.tex`), so this node depends on the PDF
/// and copies the `cells.yml` written beside it.
pub struct CellPositions {
    pub path: PathBuf,
}

impl CellPositions {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl GNode for CellPositions {
    fn tag(&self) -> String {
        "cellpositions".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn build(&self, sandbox: &Path, predecessors: &[&(dyn GNode + Send + Sync)]) -> bool {
        let [pdf] = predecessors else {
            log::error!(
                "CellPositions {} expected 1 predecessor, got {}",
                self.path.display(),
                predecessors.len()
            );
            return false;
        };
        if pdf.tag() != "pdf" {
            log::error!(
                "CellPositions {} expected a pdf predecessor, got '{}'",
                self.path.display(),
                pdf.tag()
            );
            return false;
        }

        let src = sandbox.join(pdf.pathbuf().with_file_name(CELLS_FILE));
        let dest = sandbox.join(&self.path);
        if let Some(parent) = dest.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::error!(
                "Failed to create directory for {}: {e}",
                self.path.display()
            );
            return false;
        }
        if let Err(e) = std::fs::copy(&src, &dest) {
            log::error!(
                "Failed to copy {} to {}: {e}",
                src.display(),
                dest.display()
            );
            return false;
        }
        true
    }
}
