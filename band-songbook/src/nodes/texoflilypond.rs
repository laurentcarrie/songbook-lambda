use std::path::{Path, PathBuf};
use std::process::Command;
use yamake::model::GNode;

/// Build node for the TeX output produced by LilyPond compilation.
pub struct TexOfLilypond {
    pub path: PathBuf,
}

impl TexOfLilypond {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl GNode for TexOfLilypond {
    fn tag(&self) -> String {
        "texoflilypond".to_string()
    }

    fn pathbuf(&self) -> PathBuf {
        self.path.clone()
    }

    fn build(&self, sandbox: &Path, predecessors: &[&(dyn GNode + Send + Sync)]) -> bool {
        // Find predecessors of type "lytex"
        let lytex_predecessors: Vec<_> =
            predecessors.iter().filter(|p| p.tag() == "lytex").collect();

        if lytex_predecessors.len() != 1 {
            log::error!(
                "TexOfLilypond {} expects exactly one lytex predecessor, found {}",
                self.path.display(),
                lytex_predecessors.len()
            );
            return false;
        }

        let lytex_file = lytex_predecessors[0];
        let lytex_pathbuf = lytex_file.pathbuf();
        let lytex_filename = lytex_pathbuf
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.lytex");

        // self.path = parent/xxx.output/xxx.tex
        // output_dir = parent/xxx.output
        let output_dir = match self.path.parent() {
            Some(p) => p,
            None => {
                log::error!("TexOfLilypond path has no parent: {}", self.path.display());
                return false;
            }
        };

        let output_full_path = sandbox.join(output_dir);

        // Working directory is the parent of the output directory
        let workdir = output_full_path.parent().unwrap_or(sandbox);

        log::info!(
            "Running lilypond-book on {} in {}",
            lytex_filename,
            workdir.display()
        );

        // lilypond-book caches each snippet under a hash of its own source text.
        // For \lilypondfile that is the .ly alone, so a change to a file it
        // \include-s (defs.ly) leaves the hash, and the cached PDF, unchanged.
        // This node only runs once the graph found a stale input, so start from
        // a clean output dir to make lilypond-book recompile every snippet.
        let _ = std::fs::remove_dir_all(&output_full_path);

        let mut cmd = Command::new("lilypond-book");
        cmd.arg("--output")
            .arg(output_dir.file_name().unwrap_or_default())
            .arg("--pdf")
            .arg("--latex-program=lualatex")
            .arg(lytex_filename)
            .current_dir(workdir);

        let node_id = self.path.to_string_lossy();

        // Setup logging
        let stdout_path = sandbox.join("logs").join(format!("{}.stdout", &node_id));
        let stderr_path = sandbox.join("logs").join(format!("{}.stderr", &node_id));
        let logs_dir = stdout_path.parent().unwrap_or(Path::new(""));
        let _ = std::fs::create_dir_all(logs_dir);

        let output = cmd.output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                // Save stdout and stderr for debugging
                let _ = std::fs::write(&stdout_path, &stdout);
                let _ = std::fs::write(&stderr_path, &stderr);

                if !out.status.success() {
                    log::error!(
                        "lilypond-book failed for {} (lytex: {})",
                        self.path.display(),
                        lytex_filename
                    );
                    if !stdout.is_empty() {
                        log::error!("lilypond-book stdout: {stdout}");
                    }
                    if !stderr.is_empty() {
                        log::error!("lilypond-book stderr: {stderr}");
                    }
                    return false;
                }
                true
            }
            Err(e) => {
                log::error!(
                    "Failed to run lilypond-book: {} for {}",
                    e,
                    self.path.display()
                );
                let _ = std::fs::write(&stderr_path, format!("Failed to run lilypond-book: {e}"));
                false
            }
        }
    }
}
