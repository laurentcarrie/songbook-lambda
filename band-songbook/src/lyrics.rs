//! Lyrics of a chord section, bar by bar.
//!
//! A section's lyrics are in `lyrics/<section id>.tex`. The syllable sung on the
//! first beat of a bar is tagged `\songwordfb{…}` (an empty tag for a bar
//! without one) and a pickup before it `\songwordl{…}`. The k-th `\songwordfb`
//! is the k-th bar of the section, repeats unrolled; when the numbers differ
//! the lyrics cannot be placed on the bars.

use crate::chords::bar_numbering::row_contributions;
use crate::model::{SectionItem, StructureItem};

/// The lyrics of a section do not have one `\songwordfb` per bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub bars: usize,
    pub tags: usize,
}

/// Plain text of a tag content: TeX commands and special characters dropped.
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    chars.next();
                }
            }
            '{' | '}' | '#' | '%' | '&' | '^' | '_' | '$' | '"' => {}
            '~' => out.push(' '),
            c => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `\tag{content}` occurrences in order, comments removed: (is_pickup, content).
fn tags(tex: &str) -> Vec<(bool, String)> {
    let text: String = tex
        .lines()
        .map(|l| {
            let mut cut = l.len();
            let mut prev = ' ';
            for (i, c) in l.char_indices() {
                if c == '%' && prev != '\\' {
                    cut = i;
                    break;
                }
                prev = c;
            }
            &l[..cut]
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let (pickup, name) = if rest.starts_with("\\songwordfb{") {
            (false, "\\songwordfb{")
        } else if rest.starts_with("\\songwordl{") {
            (true, "\\songwordl{")
        } else {
            i += rest.chars().next().map_or(1, |c| c.len_utf8());
            continue;
        };
        let start = i + name.len();
        let mut depth = 1;
        let mut end = start;
        for (j, c) in text[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + j;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            break; // unbalanced: stop here
        }
        out.push((pickup, plain(&text[start..end])));
        i = end + 1;
    }
    out
}

/// Number of bars of a section, repeats unrolled.
pub fn bar_total(rows: &[String]) -> usize {
    row_contributions(rows).iter().map(|c| (*c).max(0) as usize).sum()
}

/// The lyric of each bar of a section (`[pickup] syllable`, "" for none), or the
/// mismatch between the bars and the `\songwordfb` tags.
pub fn bar_lyrics(bars: usize, tex: &str) -> Result<Vec<String>, Mismatch> {
    let mut out: Vec<String> = Vec::new();
    let mut pickup = String::new();
    for (is_pickup, text) in tags(tex) {
        if is_pickup {
            if !pickup.is_empty() {
                pickup.push(' ');
            }
            pickup.push_str(&text);
        } else {
            out.push(if pickup.is_empty() {
                text
            } else {
                format!("[{pickup}] {text}").trim().to_string()
            });
            pickup.clear();
        }
    }
    if out.is_empty() {
        return Ok(out); // lyrics without first-beat tags: nothing to place, nothing to check
    }
    if out.len() == bars {
        Ok(out)
    } else {
        Err(Mismatch {
            bars,
            tags: out.len(),
        })
    }
}

/// What the lyrics of one chord or Ref section give.
pub struct SectionLyrics {
    pub id: String,
    /// Lyrics file found for the section: its lyric per bar, or the mismatch.
    pub lyrics: Option<Result<Vec<String>, Mismatch>>,
}

/// The lyrics of every Chords and Ref section of a structure. `read(id)` gives the
/// text of `lyrics/<id>.tex`, None when there is no such file. A Ref section has
/// the bars of the section it links to and its own lyrics file.
pub fn structure_lyrics(
    structure: &[StructureItem],
    read: impl Fn(&str) -> Option<String>,
) -> Vec<SectionLyrics> {
    let mut out = Vec::new();
    for item in structure {
        let bars = match &item.item {
            SectionItem::Chords(c) => bar_total(&c.rows),
            SectionItem::Ref(r) => structure
                .iter()
                .find_map(|i| match &i.item {
                    SectionItem::Chords(c) if i.id == r.link => Some(bar_total(&c.rows)),
                    _ => None,
                })
                .unwrap_or(0),
            _ => continue,
        };
        out.push(SectionLyrics {
            id: item.id.clone(),
            lyrics: read(&item.id).map(|tex| bar_lyrics(bars, &tex)),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_syllable_per_bar_with_pickup_and_empty() {
        let tex = "\\songwordl{Com}\\songwordfb{pren}drais ma belle \\\\\nA cent \\songwordfb{vingt} d\u{e9}cibels\n\\songwordfb{D\u{b4}a}mour \\songwordfb{} % \\songwordfb{x}\n";
        assert_eq!(
            bar_lyrics(4, tex),
            Ok(vec!["[Com] pren".into(), "vingt".into(), "D\u{b4}a".into(), "".into()])
        );
    }

    #[test]
    fn no_tags_is_not_a_mismatch() {
        assert_eq!(bar_lyrics(8, "Just words \\\\ and more"), Ok(vec![]));
    }

    #[test]
    fn mismatch_is_reported() {
        let tex = "\\songwordfb{a} \\songwordfb{b}";
        assert_eq!(bar_lyrics(3, tex), Err(Mismatch { bars: 3, tags: 2 }));
    }

    #[test]
    fn bars_of_a_section_unroll_repeats() {
        let rows = vec!["A|B|C|D|x2".to_string(), "E|F".to_string()];
        assert_eq!(bar_total(&rows), 10);
    }

    #[test]
    fn tex_in_a_tag_is_dropped() {
        assert_eq!(tags("\\songwordfb{\\textit{a}b}")[0].1, "ab");
    }
}
