use std::path::PathBuf;
use yamake::model::GRootNode;

/// Root node representing a ready-made pdf (`raw` in `song.yml`), delivered as is.
pub struct RawFile {
    pub path: PathBuf,
}

impl RawFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl GRootNode for RawFile {
    fn tag(&self) -> String {
        "raw".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }
}
