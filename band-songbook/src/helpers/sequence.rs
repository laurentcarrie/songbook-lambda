use std::collections::HashMap;

use strudel_of_lilypond::sequencer::model::{Bar, BarSequence, EBarSequence, SequenceItem};

use crate::chords::bar_numbering::repeat_block;
use crate::chords::parse::parse;
use crate::model::{SectionItem, StructureItem};

/// Builds a [`BarSequence`] from a song's structure, resolving `Ref`
/// sections and mapping repeats to `EBarSequence` variants.
///
/// `HRule` and `NewColumn` items are skipped since they carry no musical
/// content.
pub fn strudel_sequence_of_song(structure: &[StructureItem], tempo: u16) -> BarSequence {
    // Pre-index Chords sections so Ref lookups are O(1).
    let chords_map: HashMap<&str, &crate::model::ChordsSection> = structure
        .iter()
        .filter_map(|item| match &item.item {
            SectionItem::Chords(chords) => Some((item.id.as_str(), chords)),
            _ => None,
        })
        .collect();

    let mut sequence = Vec::new();

    for item in structure {
        match &item.item {
            SectionItem::Chords(chords) => {
                let items =
                    rows_to_ebarsequence(&chords.rows, chords.drum_sequence.as_deref(), &item.id);
                if !items.is_empty() {
                    sequence.push(SequenceItem {
                        description: chords.title.clone(),
                        item: if items.len() == 1 {
                            items.into_iter().next().unwrap()
                        } else {
                            EBarSequence::Group(items)
                        },
                    });
                }
            }
            SectionItem::Ref(ref_section) => {
                if let Some(chords) = chords_map.get(ref_section.link.as_str()) {
                    let items = rows_to_ebarsequence(
                        &chords.rows,
                        chords.drum_sequence.as_deref(),
                        &item.id,
                    );
                    if !items.is_empty() {
                        sequence.push(SequenceItem {
                            description: ref_section.title.clone(),
                            item: if items.len() == 1 {
                                items.into_iter().next().unwrap()
                            } else {
                                EBarSequence::Group(items)
                            },
                        });
                    }
                } else {
                    log::warn!(
                        "Ref section '{}' links to unknown id '{}'",
                        item.id,
                        ref_section.link
                    );
                }
            }
            SectionItem::HRule(_) | SectionItem::NewColumn => {}
        }
    }

    BarSequence {
        tempo: tempo as u32,
        sequence,
    }
}

/// Expands a `DrumSequence` into a flat list of pattern names, repeating
/// each item according to its `repeat` count.
fn expand_drum_sequence(drum_sequence: &[crate::model::DrumSequenceItem]) -> Vec<&str> {
    drum_sequence
        .iter()
        .flat_map(|item| std::iter::repeat_n(item.pattern.as_str(), item.repeat as usize))
        .collect()
}

/// Converts chord rows into a list of `EBarSequence` items.
///
/// Each row is parsed into bars. If the row has a repeat (xN), the bars are
/// wrapped in a `RepeatGroup` or `RepeatBar`. Pattern names are taken from
/// `drum_sequence` (expanded in order); bars beyond the drum sequence length
/// fall back to `"default-bar"`.
fn rows_to_ebarsequence(
    rows: &[String],
    drum_sequence: Option<&[crate::model::DrumSequenceItem]>,
    section_id: &str,
) -> Vec<EBarSequence> {
    let patterns = drum_sequence.map(expand_drum_sequence).unwrap_or_default();
    let mut pattern_idx = 0;
    let mut items = Vec::new();
    // index in `items` of the item of each row (None for a skipped row)
    let mut item_of_row: Vec<Option<usize>> = vec![None; rows.len()];

    for (row_idx, row) in rows.iter().enumerate() {
        match parse(row) {
            Ok(parsed) => {
                let bars: Vec<EBarSequence> = parsed
                    .bars
                    .iter()
                    .map(|_| {
                        let name = patterns.get(pattern_idx).copied().unwrap_or("default-bar");
                        pattern_idx += 1;
                        EBarSequence::Single(Bar {
                            pattern_name: name.to_string(),
                        })
                    })
                    .collect();

                if bars.is_empty() {
                    continue;
                }

                // `xN,M` with M > 1: the last M rows are played N times together
                let block_first = repeat_block(rows, row_idx)
                    .map(|b| b.first)
                    .filter(|&first| first < row_idx);
                let row_item = if let Some(first) = block_first {
                    let plain = if bars.len() == 1 {
                        bars.into_iter().next().unwrap()
                    } else {
                        EBarSequence::Group(bars)
                    };
                    items.push(plain);
                    let start = item_of_row[first..row_idx]
                        .iter()
                        .flatten()
                        .next()
                        .copied()
                        .unwrap_or(items.len() - 1);
                    let block: Vec<EBarSequence> = items.drain(start..).collect();
                    EBarSequence::RepeatGroup(parsed.repeat.n, block)
                } else if parsed.repeat.n > 1 {
                    if bars.len() == 1 {
                        if let EBarSequence::Single(bar) = bars.into_iter().next().unwrap() {
                            EBarSequence::RepeatBar(parsed.repeat.n, bar)
                        } else {
                            unreachable!()
                        }
                    } else {
                        EBarSequence::RepeatGroup(parsed.repeat.n, bars)
                    }
                } else if bars.len() == 1 {
                    bars.into_iter().next().unwrap()
                } else {
                    EBarSequence::Group(bars)
                };

                items.push(row_item);
                item_of_row[row_idx] = Some(items.len() - 1);
            }
            Err(e) => {
                log::warn!("Failed to parse row '{row}' in section '{section_id}': {e}");
            }
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChordsSection, DrumSequenceItem, RefSection};

    #[test]
    fn test_strudel_sequence_basic() {
        let structure = vec![
            StructureItem {
                id: "intro".to_string(),
                item: SectionItem::Chords(ChordsSection {
                    title: "Intro".to_string(),
                    section_type: "intro".to_string(),
                    section_body: None,
                    color: None,
                    rows: vec!["Am|G".to_string()],
                    drum_sequence: None,
                }),
            },
            StructureItem {
                id: "couplet1".to_string(),
                item: SectionItem::Chords(ChordsSection {
                    title: "Couplet 1".to_string(),
                    section_type: "couplet".to_string(),
                    section_body: None,
                    color: None,
                    rows: vec!["C|G|Am|F".to_string()],
                    drum_sequence: None,
                }),
            },
        ];

        let seq = strudel_sequence_of_song(&structure, 120);
        assert_eq!(seq.tempo, 120);
        assert_eq!(seq.sequence.len(), 2);
        assert_eq!(seq.sequence[0].description, "Intro");
        assert_eq!(seq.sequence[1].description, "Couplet 1");
    }

    #[test]
    fn test_strudel_sequence_with_repeat() {
        let structure = vec![StructureItem {
            id: "verse".to_string(),
            item: SectionItem::Chords(ChordsSection {
                title: "Verse".to_string(),
                section_type: "couplet".to_string(),
                section_body: None,
                color: None,
                rows: vec!["Em|G|x2".to_string()],
                drum_sequence: None,
            }),
        }];

        let seq = strudel_sequence_of_song(&structure, 140);
        assert_eq!(seq.sequence.len(), 1);
        // Should be a RepeatGroup(2, [...])
        match &seq.sequence[0].item {
            EBarSequence::RepeatGroup(n, bars) => {
                assert_eq!(*n, 2);
                assert_eq!(bars.len(), 2);
            }
            other => panic!("Expected RepeatGroup, got {other:?}"),
        }
    }

    #[test]
    fn test_strudel_sequence_with_ref() {
        let structure = vec![
            StructureItem {
                id: "intro".to_string(),
                item: SectionItem::Chords(ChordsSection {
                    title: "Intro".to_string(),
                    section_type: "intro".to_string(),
                    section_body: None,
                    color: None,
                    rows: vec!["Am|G".to_string()],
                    drum_sequence: None,
                }),
            },
            StructureItem {
                id: "refrain2".to_string(),
                item: SectionItem::Ref(RefSection {
                    title: "Refrain 2".to_string(),
                    section_type: None,
                    section_body: None,
                    color: None,
                    link: "intro".to_string(),
                }),
            },
        ];

        let seq = strudel_sequence_of_song(&structure, 120);
        assert_eq!(seq.sequence.len(), 2);
        assert_eq!(seq.sequence[0].description, "Intro");
        assert_eq!(seq.sequence[1].description, "Refrain 2");
    }

    #[test]
    fn test_strudel_sequence_skips_hrule_and_newcolumn() {
        let structure = vec![
            StructureItem {
                id: "intro".to_string(),
                item: SectionItem::Chords(ChordsSection {
                    title: "Intro".to_string(),
                    section_type: "intro".to_string(),
                    section_body: None,
                    color: None,
                    rows: vec!["Am|G".to_string()],
                    drum_sequence: None,
                }),
            },
            StructureItem {
                id: "hr".to_string(),
                item: SectionItem::HRule(100),
            },
            StructureItem {
                id: "nc".to_string(),
                item: SectionItem::NewColumn,
            },
        ];

        let seq = strudel_sequence_of_song(&structure, 120);
        assert_eq!(seq.sequence.len(), 1);
    }

    #[test]
    fn test_strudel_sequence_with_drum_sequence() {
        // 4 bars: Am|G (2 bars) + C|D (2 bars)
        // drum_sequence: 1x "rock-intro", 2x "rock-verse", 1x "fill-1"
        let structure = vec![StructureItem {
            id: "verse".to_string(),
            item: SectionItem::Chords(ChordsSection {
                title: "Verse".to_string(),
                section_type: "couplet".to_string(),
                section_body: None,
                color: None,
                rows: vec!["Am|G".to_string(), "C|D".to_string()],
                drum_sequence: Some(vec![
                    DrumSequenceItem {
                        repeat: 1,
                        pattern: "rock-intro".to_string(),
                    },
                    DrumSequenceItem {
                        repeat: 2,
                        pattern: "rock-verse".to_string(),
                    },
                    DrumSequenceItem {
                        repeat: 1,
                        pattern: "fill-1".to_string(),
                    },
                ]),
            }),
        }];

        let seq = strudel_sequence_of_song(&structure, 120);
        assert_eq!(seq.sequence.len(), 1);

        // Should be a Group of 2 groups (one per row), each with 2 bars
        fn collect_pattern_names(item: &EBarSequence) -> Vec<String> {
            match item {
                EBarSequence::Single(bar) => vec![bar.pattern_name.clone()],
                EBarSequence::Group(items) => {
                    items.iter().flat_map(collect_pattern_names).collect()
                }
                EBarSequence::RepeatBar(_, bar) => vec![bar.pattern_name.clone()],
                EBarSequence::RepeatGroup(_, items) => {
                    items.iter().flat_map(collect_pattern_names).collect()
                }
            }
        }

        let names = collect_pattern_names(&seq.sequence[0].item);
        assert_eq!(
            names,
            vec!["rock-intro", "rock-verse", "rock-verse", "fill-1"]
        );
    }
}
