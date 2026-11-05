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
    /// Delivered pdf, relative to the delivery root (`pdf/<id>.pdf`, or
    /// `raw/<file>` for a song that only has a ready-made pdf).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pdf: Option<String>,
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
    let songs: Vec<SongEntry> = world
        .items
        .iter()
        .filter_map(|(_, item)| match item {
            WorldItem::Song(song) => {
                let id = song.info.file_stem_of_song();
                let pdf = if song.structure.is_some() {
                    Some(format!("pdf/{id}.pdf"))
                } else {
                    song.raw.as_ref().map(|raw| format!("raw/{raw}"))
                };
                Some(SongEntry {
                    id,
                    author: song.info.author.clone(),
                    title: song.info.title.clone(),
                    tags: song.info.tags.clone(),
                    pdf,
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
        assert_eq!(index.books.len(), 1);
        assert_eq!(index.books[0].pdf, "pdf/book-rock.pdf");
        let yaml = world_index_yaml(&world).unwrap();
        assert_eq!(serde_yaml::from_str::<WorldIndex>(&yaml).unwrap(), index);
    }
}
