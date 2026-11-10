use super::model::{Accidental, Alteration, BarItem};

/// Renders a [`BarItem`] as a display string for TikZ chord charts.
pub fn glyph_of_baritem(item: &BarItem) -> String {
    match item {
        BarItem::Chord(chord) => {
            let mut s = format!("\\chord{}", chord.name);
            match chord.accidental {
                Accidental::Flat => s.push('f'),
                Accidental::Sharp => s.push('s'),
                Accidental::None => {}
            }
            if chord.minor {
                s.push('m');
            }
            match chord.alteration {
                Alteration::Six => s.push_str("six"),
                Alteration::Seven => s.push_str("sept"),
                Alteration::SevenSus4 => s.push_str("septsusfour"),
                Alteration::Nine => s.push_str("nine"),
                Alteration::MajorSeven => s.push_str("septM"),
                Alteration::Sus2 => s.push_str("sustwo"),
                Alteration::Sus4 => s.push_str("susfour"),
                Alteration::Dim => s.push_str("dim"),
                Alteration::Nofith => s.push_str("nofith"),
                Alteration::None => {}
            }
            // Slash chord: the bass note is drawn small under the chord
            if let Some(bass) = &chord.bass {
                let accidental = match bass.accidental {
                    Accidental::Flat => "$\\flat$",
                    Accidental::Sharp => "\\#",
                    Accidental::None => "",
                };
                s = format!("\\chordbass{{{s}}}{{{}{accidental}}}", bass.name);
            }
            s
        }
        BarItem::Rest(_) => "\\chordHRest".to_string(),
        BarItem::HalfBar => "HALFBAR".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chords::parse::parse;

    fn glyphs(row: &str) -> Vec<String> {
        parse(row)
            .unwrap()
            .bars
            .iter()
            .map(|b| glyph_of_baritem(&b.items[0]))
            .collect()
    }

    #[test]
    fn power_and_sixth_chords_have_a_macro_each() {
        assert_eq!(
            glyphs("C5|Fs5|Bb5|C6|Bf6|Fs6|Am6|Bfm6|Gsm6|C9|Bf9|Fs9|Am9|Bfm9|Gsm9"),
            [
                "\\chordCnofith",
                "\\chordFsnofith",
                "\\chordBfnofith",
                "\\chordCsix",
                "\\chordBfsix",
                "\\chordFssix",
                "\\chordAmsix",
                "\\chordBfmsix",
                "\\chordGsmsix",
                "\\chordCnine",
                "\\chordBfnine",
                "\\chordFsnine",
                "\\chordAmnine",
                "\\chordBfmnine",
                "\\chordGsmnine"
            ]
        );
    }

    #[test]
    fn seventh_sus4_chords_have_a_macro_each() {
        assert_eq!(
            glyphs("G7sus4|Bf7sus4|Fs7sus4|G7|Gsus4"),
            [
                "\\chordGseptsusfour",
                "\\chordBfseptsusfour",
                "\\chordFsseptsusfour",
                "\\chordGsept",
                "\\chordGsusfour"
            ]
        );
    }
}
