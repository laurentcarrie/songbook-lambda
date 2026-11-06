/// Book root node that expands `books/<name>.yml` into the book build graph.
pub mod bookyml;
/// Node that delivers the page position of every bar of a song PDF.
pub mod cellpositions;
/// Node that overlays click sounds onto a song MP3 for verification.
pub mod clickcheckmp3;
/// Clicks definition node with beat-level click events.
pub mod clickdef;
/// Click track analysis node that detects ticks in an MP3 and outputs tick offsets as YAML.
pub mod clickyml;
/// Node that copies a built PDF to the delivery directory.
pub mod copyfile;
/// LilyPond source file node (`.ly`).
pub mod lilypond;
/// LilyPond-to-TeX intermediate file node (`.lytex`).
pub mod lytex;
/// MIDI output of LilyPond compilation (`.midi`).
pub mod midi;
/// MP3 audio file node (`.mp3`).
pub mod mp3;
/// MP3 render synthesised from LilyPond MIDI (`.mp3`).
pub mod mp3render;
/// PDF output node, built by running `lualatex`.
pub mod pdf;
/// Standalone cropped PDF of one LilyPond file (`.pdf`).
pub mod pdfoflilypond;
/// Ready-made pdf node (`raw` in `song.yml`).
pub mod rawfile;
/// TikZ chord chart node (`song.tikz`).
pub mod songtikz;
/// Song root node that expands `song.yml` into the build graph.
pub mod songyml;
/// Strudel HTML node with interactive drum patterns.
pub mod strudel;
/// LaTeX source file node (`.tex`).
pub mod tex;
/// TeX output from LilyPond compilation.
pub mod texoflilypond;

pub use bookyml::{BookSong, BookYml};
pub use cellpositions::CellPositions;
pub use clickcheckmp3::ClickCheckMp3;
pub use clickdef::ClickDef;
pub use clickyml::ClickYml;
pub use copyfile::CopyFile;
pub use lilypond::LilypondFile;
pub use lytex::LyTexFile;
pub use midi::MidiOfLilypond;
pub use mp3::Mp3;
pub use mp3render::Mp3Render;
pub use pdf::PdfFile;
pub use pdfoflilypond::PdfOfLilypond;
pub use rawfile::RawFile;
pub use songtikz::SongTikz;
pub use songyml::SongYml;
pub use strudel::StrudelFile;
pub use tex::TexFile;
pub use texoflilypond::TexOfLilypond;
