//! The `world.yml` output: a light index of the delivered songs and books.
//!
//! It carries what a reader app needs to list and open the deliveries (names,
//! tags and the delivery path of each pdf), not the songs themselves.

use crate::model::{BookItem, World, WorldItem};
use serde::{Deserialize, Serialize};

/// File name of the index, written at the root of the delivery.
pub const WORLD_INDEX_FILE: &str = "world.yml";

/// Index of a world, serialized to `world.yml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldIndex {
    /// Sorted, deduplicated tags of the songs.
    pub tags: Vec<String>,
    /// Songs, in world order.
    pub songs: Vec<SongEntry>,
    /// Books, in world order.
    pub books: Vec<BookEntry>,
}

/// One song of the index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SongEntry {
    /// Normalized stem (`{author}--@--{title}`), as books refer to the song.
    pub id: String,
    /// Song author or band name.
    pub author: String,
    /// Song title.
    pub title: String,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Tempo in beats per minute, as declared in `song.yml`.
    pub tempo: u16,
    /// Delivered pdf, relative to the delivery root (`pdf/<id>.pdf`, or
    /// `raw/<file>` for a song that only has a ready-made pdf).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,
    /// Delivered mp3 files of the song: the recording and the renders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mp3: Vec<Mp3Entry>,
    /// Problems found in the sources of the song, shown by the apk on the song page: e.g. the
    /// lyrics of a section that do not have one `\songwordfb` per bar.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// One mp3 of a song.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mp3Entry {
    /// What it is: `song` for the recording, the score name for a render.
    pub name: String,
    /// Delivered file, relative to the delivery root (`mp3/<id>.mp3`, or
    /// `mp3-renders/<id>-<name>.mp3` for a render).
    pub path: String,
}

/// The mp3 files delivered for a song, as declared in its `song.yml`.
fn mp3_entries(id: &str, song: &crate::model::Song) -> Vec<Mp3Entry> {
    let mut out = vec![];
    if song.files.has_mp3 {
        out.push(Mp3Entry {
            name: "song".to_string(),
            path: format!("mp3/{id}.mp3"),
        });
    }
    for declared in &song.files.mp3 {
        let stem = declared.strip_suffix(".mp3").unwrap_or(declared);
        out.push(Mp3Entry {
            name: stem.to_string(),
            path: format!("mp3-renders/{id}-{stem}.mp3"),
        });
    }
    out
}

/// One book of the index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookEntry {
    /// Book name.
    pub name: String,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Ids of the songs of the book, in order.
    pub songs: Vec<String>,
    /// Delivered pdf, relative to the delivery root (`pdf/book-<name>.pdf`).
    pub pdf: String,
}

/// Builds the index of a world. Items that failed to parse are skipped.
pub fn world_index(world: &World) -> WorldIndex {
    world_index_in(world, None)
}

/// The lyrics of the sections of a song, against their bars: one warning per section
/// that does not have one `\songwordfb` per bar.
fn lyrics_warnings(song: &crate::model::Song, song_dir: &std::path::Path) -> Vec<String> {
    let Some(structure) = &song.structure else {
        return vec![];
    };
    crate::lyrics::structure_lyrics(structure, |id| {
        std::fs::read_to_string(song_dir.join("lyrics").join(format!("{id}.tex"))).ok()
    })
    .into_iter()
    .filter_map(|l| match l.lyrics {
        Some(Err(m)) => Some(format!(
            "lyrics of section '{}': {} bars but {} \\songwordfb",
            l.id, m.bars, m.tags
        )),
        _ => None,
    })
    .collect()
}

/// Same as [`world_index`], reading the lyrics of the songs in `songs_srcdir` (the directory the
/// relative paths of the world items start from) to look for warnings.
pub fn world_index_in(world: &World, songs_srcdir: Option<&std::path::Path>) -> WorldIndex {
    let songs: Vec<SongEntry> = world
        .items
        .iter()
        .filter_map(|(path, item)| match item {
            WorldItem::Song(song) => {
                let id = song.info.file_stem_of_song();
                let pdf = if song.structure.is_some() {
                    Some(format!("pdf/{id}.pdf"))
                } else {
                    song.raw.as_ref().map(|raw| format!("raw/{raw}"))
                };
                let mp3 = mp3_entries(&id, song);
                let warnings = songs_srcdir
                    .map(|dir| {
                        let song_dir = dir.join(path.parent().unwrap_or(std::path::Path::new("")));
                        lyrics_warnings(song, &song_dir)
                    })
                    .unwrap_or_default();
                Some(SongEntry {
                    id,
                    author: song.info.author.clone(),
                    title: song.info.title.clone(),
                    tags: song.info.tags.clone(),
                    tempo: song.info.tempo,
                    pdf,
                    mp3,
                    warnings,
                })
            }
            WorldItem::Error(_) => None,
        })
        .collect();

    let books = world
        .books
        .iter()
        .filter_map(|(_, item)| match item {
            BookItem::Book(book) => Some(BookEntry {
                name: book.name.clone(),
                tags: book.tags.clone(),
                songs: book.songs.clone(),
                pdf: format!("pdf/{}.pdf", book.file_stem_of_book()),
            }),
            BookItem::Error(_) => None,
        })
        .collect();

    WorldIndex {
        tags: crate::tags_of_world(world),
        songs,
        books,
    }
}

/// The index of a world as yaml text, ready to be written to `world.yml`.
pub fn world_index_yaml(world: &World) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&world_index(world))
}

/// [`world_index_yaml`] with the warnings found in the sources read from `songs_srcdir`.
pub fn world_index_yaml_in(
    world: &World,
    songs_srcdir: &std::path::Path,
) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&world_index_in(world, Some(songs_srcdir)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Book, Song};

    fn song(yaml: &str) -> Song {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn index_lists_songs_books_and_pdf_paths() {
        let with_structure = song(
            "files: {}\ninfo: {title: Dress, author: P.J. Harvey, tempo: 96, tags: [rock]}\nstructure: []\n",
        );
        let raw_only = song(
            "files: {}\ninfo: {title: Yesterday, author: Beatles, tempo: 90}\nraw: yesterday.pdf\n",
        );
        let with_mp3 = song(
            "files: {has_mp3: true, mp3: [intro.mp3, solo]}\ninfo: {title: Dress, author: P.J. Harvey, tempo: 96}\nstructure: []\n",
        );
        let mp3 = mp3_entries("id", &with_mp3);
        assert_eq!(
            mp3.iter().map(|m| m.path.as_str()).collect::<Vec<_>>(),
            vec!["mp3/id.mp3", "mp3-renders/id-intro.mp3", "mp3-renders/id-solo.mp3"]
        );
        assert_eq!(mp3[1].name, "intro");
        let book: Book = serde_yaml::from_str("name: Rock\nsongs: [a]\n").unwrap();
        let world = World {
            items: vec![
                (
                    "a/song.yml".into(),
                    WorldItem::Song(Box::new(with_structure)),
                ),
                ("b/song.yml".into(), WorldItem::Song(Box::new(raw_only))),
                ("c/song.yml".into(), WorldItem::Error("bad".into())),
            ],
            books: vec![("rock.yml".into(), BookItem::Book(Box::new(book)))],
        };

        let index = world_index(&world);

        assert_eq!(index.tags, vec!["rock".to_string()]);
        assert_eq!(index.songs.len(), 2);
        assert_eq!(
            index.songs[0].pdf.as_deref(),
            Some(format!("pdf/{}.pdf", index.songs[0].id).as_str())
        );
        assert_eq!(index.songs[1].pdf.as_deref(), Some("raw/yesterday.pdf"));
        assert_eq!(index.songs[0].tempo, 96);
        assert_eq!(index.songs[1].tempo, 90);
        assert_eq!(index.books.len(), 1);
        assert_eq!(index.books[0].pdf, "pdf/book-rock.pdf");
        let yaml = world_index_yaml(&world).unwrap();
        assert_eq!(serde_yaml::from_str::<WorldIndex>(&yaml).unwrap(), index);
    }
}
