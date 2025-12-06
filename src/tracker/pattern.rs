//! Pattern and song data structures

use serde::{Deserialize, Serialize};

/// A single note event in the tracker
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// MIDI note number (0-127), None = no note / continue
    pub pitch: Option<u8>,
    /// Instrument index (0-127), None = use previous
    pub instrument: Option<u8>,
    /// Volume (0-127), None = use previous
    pub volume: Option<u8>,
    /// Effect command (e.g., 'V' for vibrato)
    pub effect: Option<char>,
    /// Effect parameter
    pub effect_param: Option<u8>,
}

impl Note {
    pub const EMPTY: Note = Note {
        pitch: None,
        instrument: None,
        volume: None,
        effect: None,
        effect_param: None,
    };

    /// Create a note-off event
    pub fn off() -> Self {
        Self {
            pitch: Some(0xFF), // Special value for note-off
            instrument: None,
            volume: None,
            effect: None,
            effect_param: None,
        }
    }

    /// Create a note with pitch and instrument
    pub fn new(pitch: u8, instrument: u8) -> Self {
        Self {
            pitch: Some(pitch),
            instrument: Some(instrument),
            volume: None,
            effect: None,
            effect_param: None,
        }
    }

    /// Check if this is an empty slot
    pub fn is_empty(&self) -> bool {
        self.pitch.is_none()
            && self.instrument.is_none()
            && self.volume.is_none()
            && self.effect.is_none()
    }

    /// Check if this is a note-off
    pub fn is_off(&self) -> bool {
        self.pitch == Some(0xFF)
    }

    /// Format pitch as note name (e.g., "C-4", "F#5")
    pub fn pitch_name(&self) -> Option<String> {
        self.pitch.map(|p| {
            if p == 0xFF {
                "OFF".to_string()
            } else {
                let note_names = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"];
                let octave = p / 12;
                let note = (p % 12) as usize;
                format!("{}{}", note_names[note], octave)
            }
        })
    }
}

impl Default for Note {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Number of channels in the tracker
pub const NUM_CHANNELS: usize = 8;

/// Default pattern length (rows)
pub const DEFAULT_PATTERN_LEN: usize = 64;

/// A pattern is a grid of notes across channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Pattern length in rows
    pub length: usize,
    /// Notes per channel [channel][row] - using Vec for serde compatibility
    pub channels: Vec<Vec<Note>>,
}

impl Pattern {
    pub fn new(length: usize) -> Self {
        let len = length.min(256);
        Self {
            length: len,
            channels: vec![vec![Note::EMPTY; len]; NUM_CHANNELS],
        }
    }

    /// Get a note at a specific position
    pub fn get(&self, channel: usize, row: usize) -> Option<&Note> {
        self.channels.get(channel)?.get(row)
    }

    /// Set a note at a specific position
    pub fn set(&mut self, channel: usize, row: usize, note: Note) {
        if let Some(ch) = self.channels.get_mut(channel) {
            if let Some(slot) = ch.get_mut(row) {
                *slot = note;
            }
        }
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::new(DEFAULT_PATTERN_LEN)
    }
}

/// A song is a sequence of pattern indices (arrangement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    /// Song name
    pub name: String,
    /// Tempo in BPM
    pub bpm: u16,
    /// Rows per beat (typically 4)
    pub rows_per_beat: u8,
    /// All patterns in the song
    pub patterns: Vec<Pattern>,
    /// The arrangement: sequence of pattern indices
    pub arrangement: Vec<usize>,
    /// Instrument names (for display)
    pub instrument_names: Vec<String>,
}

impl Song {
    pub fn new() -> Self {
        Self {
            name: "Untitled".to_string(),
            bpm: 120,
            rows_per_beat: 4,
            patterns: vec![Pattern::default()],
            arrangement: vec![0],
            instrument_names: Vec::new(),
        }
    }

    /// Get the current pattern being edited
    pub fn current_pattern(&self, pattern_idx: usize) -> Option<&Pattern> {
        self.patterns.get(pattern_idx)
    }

    /// Get the current pattern mutably
    pub fn current_pattern_mut(&mut self, pattern_idx: usize) -> Option<&mut Pattern> {
        self.patterns.get_mut(pattern_idx)
    }

    /// Add a new pattern
    pub fn add_pattern(&mut self) -> usize {
        let idx = self.patterns.len();
        self.patterns.push(Pattern::default());
        idx
    }

    /// Calculate tick duration in seconds
    pub fn tick_duration(&self) -> f64 {
        60.0 / (self.bpm as f64 * self.rows_per_beat as f64)
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new()
    }
}

/// Effect commands (similar to MOD/XM trackers)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Effect {
    /// No effect
    None,
    /// Arpeggio (0xy)
    Arpeggio(u8, u8),
    /// Slide up (1xx)
    SlideUp(u8),
    /// Slide down (2xx)
    SlideDown(u8),
    /// Portamento to note (3xx)
    Portamento(u8),
    /// Vibrato (4xy)
    Vibrato(u8, u8),
    /// Volume slide (Axy)
    VolumeSlide(u8, u8),
    /// Set volume (Cxx)
    SetVolume(u8),
    /// Pattern break (Dxx)
    PatternBreak(u8),
    /// Set speed (Fxx)
    SetSpeed(u8),
}

impl Effect {
    /// Parse effect from character and parameter
    pub fn from_char(c: char, param: u8) -> Self {
        match c.to_ascii_uppercase() {
            '0' => Effect::Arpeggio(param >> 4, param & 0x0F),
            '1' => Effect::SlideUp(param),
            '2' => Effect::SlideDown(param),
            '3' => Effect::Portamento(param),
            '4' => Effect::Vibrato(param >> 4, param & 0x0F),
            'A' => Effect::VolumeSlide(param >> 4, param & 0x0F),
            'C' => Effect::SetVolume(param),
            'D' => Effect::PatternBreak(param),
            'F' => Effect::SetSpeed(param),
            _ => Effect::None,
        }
    }
}
