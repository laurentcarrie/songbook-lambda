use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};

/// A complete song definition, deserialized from `song.yml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Song {
    /// Source files (LilyPond, TeX, WAV) associated with this song.
    pub files: SongFiles,
    /// Title, author, tempo, and time signature.
    pub info: SongInfo,
    /// Optional metadata such as date and content digest.
    #[serde(default)]
    pub meta: SongMeta,
    /// Ordered list of sections that make up the song structure.
    pub structure: Vec<StructureItem>,
}

/// Source file references for a song.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongFiles {
    /// LilyPond notation files (`.ly`).
    #[serde(default)]
    pub lilypond: Vec<String>,
    /// LaTeX source files (`.tex`).
    #[serde(default)]
    pub tex: Vec<String>,
    /// Audio renders (`.mp3`), synthesised from the matching `.ly`.
    #[serde(default)]
    pub mp3: Vec<String>,
    /// Whether this song has a click track (`clicks.mp3`).
    #[serde(default)]
    pub has_clicks: bool,
    /// Whether this song has a song recording (`song.mp3`).
    #[serde(default)]
    pub has_mp3: bool,
}

/// Core song metadata: title, author, tempo, and time signature.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SongInfo {
    /// Song title.
    pub title: String,
    /// Song author or band name.
    pub author: String,
    /// Tempo in beats per minute.
    pub tempo: u16,
    /// Time signature (e.g. `"4/4"`), if specified.
    pub time_signature: Option<String>,
    /// Free-form tags for categorizing the song.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Identifier of the song on an external music service, if known.
    #[serde(default)]
    pub external_id: Option<ExternalId>,
}

/// The identifier of a song on an external music service.
///
/// In yaml, the variant is given as a tag: `external_id: !Deezer "3135556"`
/// or `external_id: !Youtube "dQw4w9WgXcQ"`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ExternalId {
    /// A Deezer track id.
    Deezer(String),
    /// A YouTube video id.
    Youtube(String),
}

/// Normalizes a name for use in a file name.
/// - Capitals are replaced with lowercase
/// - Accented Latin letters fold to their ASCII base (é -> e, ß -> ss)
/// - All remaining characters which are not a-z0-9 are replaced with '_'
pub fn normalize_name(s: &str) -> String {
    // Accented letters fold to their ASCII base rather than to '_', so
    // "Noir Désir" is noir_desir and not noir_d_sir.
    let mut out = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        match c {
            'a'..='z' | '0'..='9' => out.push(c),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => out.push('a'),
            'ç' => out.push('c'),
            'è' | 'é' | 'ê' | 'ë' => out.push('e'),
            'ì' | 'í' | 'î' | 'ï' => out.push('i'),
            'ñ' => out.push('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => out.push('o'),
            'ù' | 'ú' | 'û' | 'ü' => out.push('u'),
            'ý' | 'ÿ' => out.push('y'),
            'ð' => out.push('d'),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            'ß' => out.push_str("ss"),
            'þ' => out.push_str("th"),
            _ => out.push('_'),
        }
    }
    out
}

impl SongInfo {
    /// Returns a normalized file stem based on author and title.
    /// Format: `{author}--@--{title}`, each part normalized by [`normalize_name`].
    pub fn file_stem_of_song(&self) -> String {
        format!(
            "{}--@--{}",
            normalize_name(&self.author),
            normalize_name(&self.title)
        )
    }
}

/// Optional metadata for a song (date, content digest).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SongMeta {
    /// Date the song was added or last modified.
    #[serde(default)]
    pub date: Option<String>,
    /// MD5 digest of the song content for change detection.
    #[serde(default)]
    pub digest: Option<String>,
}

/// A named section in the song structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StructureItem {
    /// Unique identifier for this section (e.g. `"intro"`, `"couplet1"`).
    pub id: String,
    /// The section content.
    pub item: SectionItem,
}

/// The different kinds of sections that can appear in a song structure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SectionItem {
    /// A section with chord rows.
    Chords(ChordsSection),
    /// A reference to another section (shares its chord content).
    Ref(RefSection),
    /// A horizontal rule with a given width percentage.
    HRule(u32),
    /// A column break in multi-column layouts.
    NewColumn,
}

/// A section containing chord rows and optional drum patterns.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChordsSection {
    /// Display title for this section.
    pub title: String,
    /// Section type (e.g. `"intro"`, `"couplet"`, `"refrain"`).
    #[serde(rename = "type")]
    pub section_type: String,
    /// Optional body text displayed below the section header.
    pub section_body: Option<String>,
    /// Background color name (X11 color).
    pub color: Option<String>,
    /// Chord rows, each in `"A|Bm|C#|x2"` format.
    pub rows: Vec<String>,
    /// Optional drum pattern sequence for Strudel generation.
    pub drum_sequence: Option<DrumSequence>,
}

/// A single item in a drum sequence: a pattern name repeated N times.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DrumSequenceItem {
    /// Number of consecutive bars using this pattern.
    pub repeat: u32,
    /// Name of the drum pattern (must exist in a library directory).
    pub pattern: String,
}

/// A sequence of drum pattern items for a section.
pub type DrumSequence = Vec<DrumSequenceItem>;

/// A section that references another section's chord content.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefSection {
    /// Display title for this reference section.
    pub title: String,
    /// Optional override of the section type.
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    /// Optional body text.
    pub section_body: Option<String>,
    /// Background color name (X11 color).
    pub color: Option<String>,
    /// ID of the [`ChordsSection`] this references.
    pub link: String,
}

/// A book: a named, ordered selection of songs, deserialized from a yaml
/// file in the `books/` directory of the source tree.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Book {
    /// Book name, used for the delivered file name (`book-<name>.pdf`).
    pub name: String,
    /// Free-form tags for categorizing the book.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Songs in the book, in the order they are collated, each given by its
    /// normalized stem (`{author}--@--{title}`, see [`SongInfo::file_stem_of_song`]).
    #[serde(default)]
    pub songs: Vec<String>,
}

impl Book {
    /// Returns the normalized file stem of the book: `book-{name}`.
    pub fn file_stem_of_book(&self) -> String {
        format!("book-{}", normalize_name(&self.name))
    }
}

/// An item in the world's book list: either a successfully parsed book or an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookItem {
    /// A successfully parsed book.
    Book(Box<Book>),
    /// An error message from reading or parsing a book file.
    Error(String),
}

/// An item in the world: either a successfully parsed song or an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldItem {
    /// A successfully parsed song.
    Song(Box<Song>),
    /// An error message from reading or parsing a song file.
    Error(String),
}

/// Click offsets parsed from `clicks.yml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clicks {
    /// Absolute time offsets (in seconds) of each detected click from the start of the audio.
    #[serde(alias = "ticks")]
    pub clicks: Vec<f64>,
}

/// A single click event with its bar/beat position, time, and description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Click {
    /// Bar number (1-based).
    pub bar_number: u32,
    /// Beat within the bar (1-based, e.g. 1–4 for 4/4 time).
    pub beat_in_bar_number: u32,
    /// Absolute time in seconds from the start of the audio.
    /// Deserialized from `"mm:ss.ms"` format (e.g. `"1:30.5"` = 90.5 seconds).
    #[serde(deserialize_with = "deserialize_time")]
    pub time: f64,
    /// Description of this click (e.g. bar number, section name).
    pub description: String,
}

/// A raw click event as read from `clicks-def.yml`, before section-id resolution.
///
/// The bar position can be specified either as an explicit `bar_number` or as a
/// `section_id` referencing a section in `song.yml` (resolved to its first bar).
/// Exactly one of `bar_number` or `section_id` must be set.
#[derive(Debug, Clone, Deserialize)]
pub struct RawClick {
    /// Explicit bar number (1-based). Mutually exclusive with `section_id`.
    pub bar_number: Option<u32>,
    /// Section id whose first bar is used. Mutually exclusive with `bar_number`.
    pub section_id: Option<String>,
    /// Beat within the bar (1-based, e.g. 1–4 for 4/4 time). Defaults to 1 if omitted.
    #[serde(default = "default_beat_in_bar_number")]
    pub beat_in_bar_number: u32,
    /// Absolute time in seconds from the start of the audio.
    #[serde(deserialize_with = "deserialize_time")]
    pub time: f64,
    /// Description of this click (optional, defaults to empty string).
    #[serde(default)]
    pub description: String,
}

/// A full click track definition as read from `clicks-def.yml` (before resolution).
#[derive(Debug, Clone, Deserialize)]
pub struct RawClicksDefinition {
    /// Ordered list of raw click events.
    pub clicks: Vec<RawClick>,
}

fn default_beat_in_bar_number() -> u32 {
    1
}

/// Parse a time string in `"mm:ss.ms"` format into seconds.
fn parse_time(s: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    // A leading '-' negates the whole value: "-00:02.00" is -2 s. Parsing the
    // minutes on their own would lose the sign, since "-00" is just -0.
    let (sign, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1.0, rest),
        None => (1.0, trimmed),
    };
    let (minutes, rest) = body
        .split_once(':')
        .ok_or_else(|| format!("expected mm:ss.ms format, got {s}"))?;
    let minutes: f64 = minutes
        .parse()
        .map_err(|e| format!("invalid minutes in {s}: {e}"))?;
    let seconds: f64 = rest
        .parse()
        .map_err(|e| format!("invalid seconds in {s}: {e}"))?;
    if minutes < 0.0 || seconds < 0.0 {
        return Err(format!(
            "negative component in {s}: put the sign before the minutes, as -mm:ss.ms"
        ));
    }
    Ok(sign * (minutes * 60.0 + seconds))
}

fn deserialize_time<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_time(&s).map_err(serde::de::Error::custom)
}

/// A full click track definition with all click events (bar numbers resolved).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClicksDefinition {
    /// Ordered list of click events.
    pub clicks: Vec<Click>,
}

const TICKS_PER_BAR: usize = 4;

/// Returns the start time (in seconds) of bar `bar` (1-based).
///
/// If `clicks` is provided, the time is looked up from the ticks array (4 ticks per bar).
/// Otherwise, it is computed from the tempo: `(bar - 1) * 4 * 60.0 / tempo`.
pub fn time_of_bar(bar: u32, tempo: u16, clicks: Option<&Clicks>) -> f64 {
    match clicks {
        Some(cd) => {
            let tick_index = (bar as usize - 1) * TICKS_PER_BAR;
            cd.clicks[tick_index]
        }
        None => (bar as f64 - 1.0) * TICKS_PER_BAR as f64 * 60.0 / tempo as f64,
    }
}

/// The world: a collection of song paths and their parse results,
/// together with the books that collate them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    /// List of (relative path, world item) pairs.
    pub items: Vec<(PathBuf, WorldItem)>,
    /// List of (relative path, book item) pairs, one per yaml file in `books/`.
    #[serde(default)]
    pub books: Vec<(PathBuf, BookItem)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stem(author: &str, title: &str) -> String {
        SongInfo {
            title: title.to_string(),
            author: author.to_string(),
            tempo: 120,
            time_signature: None,
            tags: vec![],
            external_id: None,
        }
        .file_stem_of_song()
    }

    #[test]
    fn test_file_stem_folds_accents_to_ascii() {
        assert_eq!(stem("Noir Désir", "Marlène"), "noir_desir--@--marlene");
        assert_eq!(
            stem("Axel Bauer", "Eteins La Lumière"),
            "axel_bauer--@--eteins_la_lumiere"
        );
        assert_eq!(stem("Motörhead", "Ça Ira"), "motorhead--@--ca_ira");
    }

    #[test]
    fn test_file_stem_keeps_ascii_behaviour() {
        assert_eq!(
            stem("Oasis", "Rock 'N Roll Star"),
            "oasis--@--rock__n_roll_star"
        );
        assert_eq!(stem("Maroon5", "This Love"), "maroon5--@--this_love");
    }

    #[test]
    fn test_external_id_yaml_roundtrip() {
        let info: SongInfo = serde_yaml::from_str(
            "title: Dress\nauthor: P.J. Harvey\ntempo: 96\ntime_signature: null\nexternal_id: !Deezer \"3135556\"\n",
        )
        .unwrap();
        assert_eq!(info.external_id, Some(ExternalId::Deezer("3135556".to_string())));

        let info: SongInfo = serde_yaml::from_str(
            "title: Dress\nauthor: P.J. Harvey\ntempo: 96\ntime_signature: null\nexternal_id: !Youtube \"dQw4w9WgXcQ\"\n",
        )
        .unwrap();
        assert_eq!(
            info.external_id,
            Some(ExternalId::Youtube("dQw4w9WgXcQ".to_string()))
        );
    }

    #[test]
    fn test_external_id_is_optional() {
        // songs written before the field exists must still parse
        let info: SongInfo =
            serde_yaml::from_str("title: Dress\nauthor: P.J. Harvey\ntempo: 96\ntime_signature: null\n")
                .unwrap();
        assert_eq!(info.external_id, None);
    }

    #[test]
    fn test_parse_time_negative_applies_to_whole_value() {
        // the sign must survive a zero minutes field
        assert_eq!(parse_time("-00:02.00"), Ok(-2.0));
        assert_eq!(parse_time("-01:30.5"), Ok(-90.5));
        assert_eq!(parse_time("00:02.00"), Ok(2.0));
        assert_eq!(parse_time("1:30.5"), Ok(90.5));
        assert_eq!(parse_time(" 0:0.0 "), Ok(0.0));
    }

    #[test]
    fn test_parse_time_rejects_inner_sign() {
        assert!(parse_time("00:-02.00").is_err());
        assert!(parse_time("nope").is_err());
    }

    #[test]
    fn test_time_of_bar_without_clicks() {
        // 120 BPM, 4 ticks per bar => each bar = 4 * 60/120 = 2.0 seconds
        assert!((time_of_bar(1, 120, None) - 0.0).abs() < 1e-9);
        assert!((time_of_bar(2, 120, None) - 2.0).abs() < 1e-9);
        assert!((time_of_bar(3, 120, None) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_time_of_bar_with_clicks() {
        // 8 ticks = 2 bars of 4 ticks each
        let clicks = Clicks {
            clicks: vec![0.0, 0.428, 0.857, 1.285, 1.714, 2.142, 2.571, 3.0],
        };
        assert!((time_of_bar(1, 140, Some(&clicks)) - 0.0).abs() < 1e-9);
        assert!((time_of_bar(2, 140, Some(&clicks)) - 1.714).abs() < 1e-9);
    }
}
