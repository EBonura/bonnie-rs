//! Tracker/Music Editor
//!
//! A pattern-based music tracker with SF2 soundfont support.
//! Inspired by Picotron's tracker design.

mod state;
mod audio;
mod pattern;
mod layout;

pub use state::TrackerState;
pub use audio::AudioEngine;
pub use pattern::*;
pub use layout::draw_tracker;
