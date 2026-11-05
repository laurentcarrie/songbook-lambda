//! Bar numbering utilities for chord charts.

use std::collections::HashMap;

use super::parse::parse;
use crate::model::{SectionItem, StructureItem};

/// The repeated block ending at row `idx`, if that row carries a repeat marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatBlock {
    /// Number of times the block is played in total (>= 2).
    pub n: u32,
    /// Index of the first row of the block (`last` is the row with the marker).
    pub first: usize,
    pub last: usize,
}

/// Number of bars of each row (0 for a row that does not parse).
pub fn row_bar_counts(rows: &[String]) -> Vec<usize> {
    rows.iter()
        .map(|r| parse(r).map(|p| p.bars.len()).unwrap_or(0))
        .collect()
}

/// The repeat block ending at row `idx` (`xN` = this row, `xN,M` = the M rows
/// ending here). None when the row has no repeat. A block longer than the
/// rows above it is cut at the first row.
pub fn repeat_block(rows: &[String], idx: usize) -> Option<RepeatBlock> {
    let repeat = parse(rows.get(idx)?).ok()?.repeat;
    if repeat.n <= 1 {
        return None;
    }
    let first = (idx + 1).saturating_sub(repeat.rows as usize);
    Some(RepeatBlock {
        n: repeat.n,
        first,
        last: idx,
    })
}

/// Bars each row adds to the running bar count: its own bars, plus, for the
/// last row of a repeated block, the extra passes of the whole block.
pub fn row_contributions(rows: &[String]) -> Vec<i32> {
    let bars = row_bar_counts(rows);
    (0..rows.len())
        .map(|i| {
            let extra = repeat_block(rows, i)
                .map(|b| (b.n as usize - 1) * bars[b.first..=b.last].iter().sum::<usize>())
                .unwrap_or(0);
            (bars[i] + extra) as i32
        })
        .collect()
}

/// Computes bar numbers for sections.
///
/// Returns a HashMap where:
/// - Key: section id
/// - Value: (Vec of bar numbers for each row, total bar count for the section)
pub fn barcount_map_of_structure(structure: &[StructureItem]) -> HashMap<String, (Vec<i32>, i32)> {
    let mut result = HashMap::new();
    let mut current_bar_count = 1;

    // Build a map of section id -> rows for Chords sections
    let chords_rows: HashMap<String, &Vec<String>> = structure
        .iter()
        .filter_map(|item| match &item.item {
            SectionItem::Chords(chords) => Some((item.id.clone(), &chords.rows)),
            _ => None,
        })
        .collect();

    for item in structure {
        match &item.item {
            SectionItem::Chords(chords) => {
                let mut row_numbers = Vec::new();
                for add in row_contributions(&chords.rows) {
                    row_numbers.push(current_bar_count);
                    current_bar_count += add;
                }
                result.insert(item.id.clone(), (row_numbers, current_bar_count));
            }
            SectionItem::Ref(ref_section) => {
                let mut row_numbers = Vec::new();
                if let Some(rows) = chords_rows.get(&ref_section.link) {
                    for add in row_contributions(rows) {
                        row_numbers.push(current_bar_count);
                        current_bar_count += add;
                    }
                }
                result.insert(item.id.clone(), (row_numbers, current_bar_count));
            }
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_repeat_block_and_contributions() {
        // 4 rows of 2 bars; rows 0-1 played twice (marker on row 1)
        let r = rows(&["A|B", "C|D|x2,2", "E|F"]);
        assert_eq!(
            repeat_block(&r, 1),
            Some(RepeatBlock {
                n: 2,
                first: 0,
                last: 1
            })
        );
        assert_eq!(repeat_block(&r, 0), None);
        // row 1 adds its 2 bars + 1 extra pass of the 4-bar block
        assert_eq!(row_contributions(&r), vec![2, 2 + 4, 2]);
        // single-row repeat keeps the previous meaning
        assert_eq!(row_contributions(&rows(&["A|B|x3"])), vec![6]);
        // a block longer than the rows above is cut at the first row
        assert_eq!(repeat_block(&rows(&["A|x2,5"]), 0).unwrap().first, 0);
    }
    use crate::model::{ChordsSection, RefSection};

    #[test]
    fn test_barcount_map_of_structure() {
        let structure = vec![
            StructureItem {
                id: "intro".to_string(),
                item: SectionItem::Chords(ChordsSection {
                    title: "Intro".to_string(),
                    section_type: "intro".to_string(),
                    section_body: None,
                    color: None,
                    rows: vec!["Am|G".to_string(), "F|E".to_string()], // 2 bars each row
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
                    rows: vec!["C|G|Am|F".to_string(), "C|G|x2".to_string()], // 4 bars, then 2 bars x2
                    drum_sequence: None,
                }),
            },
            StructureItem {
                id: "refrain1".to_string(),
                item: SectionItem::Ref(RefSection {
                    title: "Refrain 1".to_string(),
                    section_type: None,
                    section_body: None,
                    color: None,
                    link: "intro".to_string(), // links to intro (2 bars + 2 bars)
                }),
            },
        ];

        let result = barcount_map_of_structure(&structure);

        // intro: row 0 starts at 1, row 1 starts at 3 (1 + 2 bars)
        // total after intro: 1 + 2 + 2 = 5
        let intro = result.get("intro").unwrap();
        assert_eq!(intro.0, vec![1, 3]);
        assert_eq!(intro.1, 5);

        // couplet1: row 0 starts at 5, row 1 starts at 9 (5 + 4 bars)
        // total after couplet1: 5 + 4 + (2 * 2) = 13
        let couplet1 = result.get("couplet1").unwrap();
        assert_eq!(couplet1.0, vec![5, 9]);
        assert_eq!(couplet1.1, 13);

        // refrain1 (ref to intro): row 0 starts at 13, row 1 starts at 15
        // total after refrain1: 13 + 2 + 2 = 17
        let refrain1 = result.get("refrain1").unwrap();
        assert_eq!(refrain1.0, vec![13, 15]);
        assert_eq!(refrain1.1, 17);
    }
}
