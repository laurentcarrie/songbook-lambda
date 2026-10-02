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
