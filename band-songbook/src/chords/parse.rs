use super::model::{Accidental, Alteration, Bar, BarItem, Bass, Chord, ParsedRow, Repeat, Rest};

/// Error type for chord parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidChord(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidChord(s) => write!(f, "Invalid chord: {s}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a chord string into a BarItem
/// Valid formats: [A-G][m]?[7M?]? (e.g., C, Am, G7, Em7, C7M)
fn parse_item(s: &str) -> Result<BarItem, ParseError> {
    let s = s.trim();

    // Check for HRest
    if s == "HRest" {
        return Ok(BarItem::Rest(Rest { duration: 1 }));
    }

    // Check for half-bar marker
    if s == "2/4" {
        return Ok(BarItem::HalfBar);
    }

    // Slash chord: <chord>/<bass note>, e.g. B/D# or Bs/Ds
    if let Some((chord_part, bass_part)) = s.split_once('/') {
        let bass = parse_bass(bass_part).ok_or_else(|| ParseError::InvalidChord(s.to_string()))?;
        return match parse_item(chord_part)? {
            BarItem::Chord(chord) => Ok(BarItem::Chord(Chord {
                bass: Some(bass),
                ..chord
            })),
            _ => Err(ParseError::InvalidChord(s.to_string())),
        };
    }

    let mut chars = s.chars().peekable();

    // First letter must be A-G
    let name = chars
        .next()
        .filter(|c| ('A'..='G').contains(c))
        .map(|c| c.to_string())
        .ok_or_else(|| ParseError::InvalidChord(s.to_string()))?;

    // Check for accidental (# or s for sharp, b or f for flat)
    // Note: 's' for sharp must not be followed by 'u' (to avoid confusion with 'sus')
    let accidental = match chars.peek() {
        Some('#') => {
            chars.next();
            Accidental::Sharp
        }
        Some('s') => {
            let mut lookahead = chars.clone();
            lookahead.next();
            if lookahead.peek() == Some(&'u') {
                Accidental::None // It's 'sus', not sharp
            } else {
                chars.next();
                Accidental::Sharp
            }
        }
        Some('b') | Some('f') => {
            chars.next();
            Accidental::Flat
        }
        _ => Accidental::None,
    };

    // Check for minor 'm'
    let minor = if chars.peek() == Some(&'m') {
        chars.next();
        true
    } else {
        false
    };

    // Check for alteration: '6', '7' optionally followed by 'M', 'dim', 'sus2', 'sus4'
    let alteration = if chars.peek() == Some(&'6') {
        chars.next();
        Alteration::Six
    } else if chars.peek() == Some(&'7') {
        chars.next();
        if chars.peek() == Some(&'M') {
            chars.next();
            Alteration::MajorSeven
        } else {
            Alteration::Seven
        }
    } else if chars.peek() == Some(&'d') || chars.peek() == Some(&'s') {
        let suffix: String = chars.by_ref().collect();
        match suffix.as_str() {
            "dim" => Alteration::Dim,
            "sus2" => Alteration::Sus2,
            "sus4" => Alteration::Sus4,
            _ => return Err(ParseError::InvalidChord(s.to_string())),
        }
    } else {
        Alteration::None
    };

    // If there are remaining characters, it's invalid
    if chars.next().is_some() {
        return Err(ParseError::InvalidChord(s.to_string()));
    }

    Ok(BarItem::Chord(Chord {
        name,
        accidental,
        minor,
        alteration,
        bass: None,
    }))
}

/// Parses the bass note of a slash chord: [A-G] optionally followed by
/// # or s (sharp), b or f (flat)
fn parse_bass(s: &str) -> Option<Bass> {
    let mut chars = s.chars();
    let name = chars.next().filter(|c| ('A'..='G').contains(c))?;
    let accidental = match chars.next() {
        None => Accidental::None,
        Some('#') | Some('s') => Accidental::Sharp,
        Some('b') | Some('f') => Accidental::Flat,
        Some(_) => return None,
    };
    if chars.next().is_some() {
        return None;
    }
    Some(Bass {
        name: name.to_string(),
        accidental,
    })
}

/// Parses a string of chords separated by | into a ParsedRow
/// Each bar contains chords separated by spaces
/// Empty bars are filtered out
/// Repeat markers (xN) at the end are parsed
pub fn parse(input: &str) -> Result<ParsedRow, ParseError> {
    let parts: Vec<&str> = input.split('|').collect();

    // Check if the last part is a repeat marker (xN)
    let (bar_parts, repeat) = if let Some(last) = parts.last() {
        let trimmed = last.trim();
        if let Some(stripped) = trimmed.strip_prefix('x') {
            if let Ok(n) = stripped.parse::<u32>() {
                (&parts[..parts.len() - 1], Repeat { n })
            } else {
                (&parts[..], Repeat { n: 1 })
            }
        } else {
            (&parts[..], Repeat { n: 1 })
        }
    } else {
        (&parts[..], Repeat { n: 1 })
    };

    let bars: Result<Vec<Bar>, ParseError> = bar_parts
        .iter()
        .map(|bar_str| {
            let items: Result<Vec<BarItem>, ParseError> =
                bar_str.split_whitespace().map(parse_item).collect();
            Ok(Bar { items: items? })
        })
        .collect();

    let bars = bars?
        .into_iter()
        .filter(|bar| !bar.items.is_empty())
        .collect();

    Ok(ParsedRow { bars, repeat })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let input = "Em | Em|C7|G|HRest";
        let result = parse(input).unwrap();
        assert_eq!(result.bars.len(), 5);
        assert_eq!(result.repeat.n, 1);

        // Em - minor chord
        if let BarItem::Chord(chord) = &result.bars[0].items[0] {
            assert_eq!(chord.name, "E");
            assert!(chord.minor);
            assert_eq!(chord.alteration, Alteration::None);
        } else {
            panic!("Expected Chord");
        }

        // C7 - seventh
        if let BarItem::Chord(chord) = &result.bars[2].items[0] {
            assert_eq!(chord.name, "C");
            assert!(!chord.minor);
            assert_eq!(chord.alteration, Alteration::Seven);
        } else {
            panic!("Expected Chord");
        }

        // G - major chord
        if let BarItem::Chord(chord) = &result.bars[3].items[0] {
            assert_eq!(chord.name, "G");
            assert!(!chord.minor);
            assert_eq!(chord.alteration, Alteration::None);
        } else {
            panic!("Expected Chord");
        }

        // HRest
        if let BarItem::Rest(rest) = &result.bars[4].items[0] {
            assert_eq!(rest.duration, 1);
        } else {
            panic!("Expected Rest");
        }
    }

    #[test]
    fn test_parse_with_repeat() {
        let input = "Em|G|Am|C7|x2";
        let result = parse(input).unwrap();
        assert_eq!(result.bars.len(), 4);
        assert_eq!(result.repeat.n, 2);
    }

    #[test]
    fn test_parse_slash_chord() {
        let result = parse("B/D#|Em/B|C/Gf|G").unwrap();
        assert_eq!(result.bars.len(), 4);
        if let BarItem::Chord(chord) = &result.bars[0].items[0] {
            assert_eq!(chord.name, "B");
            let bass = chord.bass.as_ref().unwrap();
            assert_eq!(bass.name, "D");
            assert_eq!(bass.accidental, Accidental::Sharp);
        } else {
            panic!("Expected Chord");
        }
        if let BarItem::Chord(chord) = &result.bars[2].items[0] {
            assert_eq!(chord.bass.as_ref().unwrap().accidental, Accidental::Flat);
        } else {
            panic!("Expected Chord");
        }
        assert!(parse("B/H").is_err());
    }

    #[test]
    fn test_parse_invalid() {
        let input = "X";
        let result = parse(input);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseError::InvalidChord("X".to_string())
        );
    }
}
