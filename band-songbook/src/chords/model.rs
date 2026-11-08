/// Indicates an accidental (flat, sharp, or none)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Accidental {
    Flat,
    Sharp,
    None,
}

/// Indicates a chord alteration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alteration {
    None,
    Nofith,
    Six,
    Seven,
    MajorSeven,
    Nine,
    Sus2,
    Sus4,
    Dim,
}

/// The bass note of a slash chord (the `D#` in `B/D#`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bass {
    pub name: String,
    pub accidental: Accidental,
}

/// Represents a musical chord
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub name: String,
    pub accidental: Accidental,
    pub minor: bool,
    pub alteration: Alteration,
    /// Bass note of a slash chord, if any
    pub bass: Option<Bass>,
}

/// Represents a rest (silence)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rest {
    pub duration: u32,
}

/// Represents a repeat marker: `xN` repeats the row N times in total,
/// `xN,M` repeats the M rows ending at this row N times in total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repeat {
    /// Number of times the block is played (1 = no repeat).
    pub n: u32,
    /// Number of rows of the block, this row being the last one (>= 1).
    pub rows: u32,
}

/// An item in a bar: either a chord, a rest, or a half-bar marker (2/4)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarItem {
    Chord(Chord),
    Rest(Rest),
    /// Half-bar marker: the right triangle of the cell is grayed out.
    HalfBar,
}

/// Represents a bar (measure) containing chords with their durations
#[derive(Debug, Clone)]
pub struct Bar {
    pub items: Vec<BarItem>,
}

/// Represents a parsed row with bars and an optional repeat
#[derive(Debug, Clone)]
pub struct ParsedRow {
    pub bars: Vec<Bar>,
    pub repeat: Repeat,
}
