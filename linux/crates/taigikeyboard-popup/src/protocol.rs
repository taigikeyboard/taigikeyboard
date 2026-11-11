//! One JSON object per line, both directions. The parent writes commands
//! on the child's stdin; the child writes events on stdout.

use serde::{Deserialize, Serialize};

/// Where the caret is, in screen pixels. An empty rect (`w` or `h` ≤ 0)
/// still places the window: the top-left of the screen plus a gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caret {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// One cell. `annotation` is the other script; absent for a one-script cell
/// (a literal, a symbol, a Telex-guide row).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub text: String,
    pub annotation: Option<String>,
}

/// Colors and typefaces for one page. Colors are `0xRRGGBB`. An empty
/// family is the system sans. Missing fields on the wire keep [`Default`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chrome {
    /// Window pad in px, before scale. Cell pad, gap, line gap and the
    /// stacked-script gap follow it.
    #[serde(default = "default_padding")]
    pub padding: u32,
    /// Corner radius in px, before scale. Zero is square.
    #[serde(default)]
    pub radius: u32,
    #[serde(default)]
    pub hanji_family: String,
    #[serde(default)]
    pub roman_family: String,
    #[serde(default = "default_text")]
    pub text_color: u32,
    #[serde(default = "default_selected_text")]
    pub selected_text_color: u32,
    #[serde(default = "default_background")]
    pub background_color: u32,
    #[serde(default = "default_selected_background")]
    pub selected_background_color: u32,
    /// How full the highlight is, 0–100. `0` is the cell. `100` meets the
    /// window's outer edge.
    #[serde(default)]
    pub highlight_fill: u32,
    /// The four colors above are the fallback. The window replaces them
    /// with the desktop's color scheme and accent when it can read them.
    /// A named theme or custom colors leave this false.
    #[serde(default)]
    pub follow_system: bool,
}

fn default_padding() -> u32 {
    4
}

fn default_text() -> u32 {
    0x1A_1A_1A
}

fn default_selected_text() -> u32 {
    0x00FF_FFFF
}

fn default_background() -> u32 {
    0x00F6_F5F4
}

fn default_selected_background() -> u32 {
    0x001A_5FB4
}

impl Default for Chrome {
    fn default() -> Self {
        Self {
            padding: default_padding(),
            radius: 4,
            hanji_family: String::new(),
            roman_family: String::new(),
            text_color: default_text(),
            selected_text_color: default_selected_text(),
            background_color: default_background(),
            selected_background_color: default_selected_background(),
            highlight_fill: 50,
            follow_system: false,
        }
    }
}

/// One page, already sliced by the parent. `cursor` is the index on this
/// page, or absent when nothing is highlighted (the Telex guide).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Show {
    pub cells: Vec<Cell>,
    pub labels: Vec<String>,
    pub cursor: Option<u32>,
    pub vertical: bool,
    /// A page exists before this one.
    pub has_prev: bool,
    /// A page exists after this one.
    pub has_next: bool,
    /// Primary size in px, before the window's own scale. The leading line
    /// of a paired cell, and a one-script cell in the mode's leading script.
    /// The settings key is still `hanjiFontSize`.
    pub hanji_px: u32,
    /// Secondary size in px, before the window's own scale. The settings key
    /// is still `romanFontSize`.
    pub roman_px: u32,
    /// The leading script of a two-script cell is Hanji.
    pub hanji_leads: bool,
    #[serde(flatten)]
    pub chrome: Chrome,
    #[serde(flatten)]
    pub caret: Caret,
}

/// What the parent asks. A `show` replaces the page and moves the window;
/// a `move` only moves it; `hide` takes it off screen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum Command {
    Show {
        #[serde(flatten)]
        body: Show,
    },
    Hide,
    Move {
        #[serde(flatten)]
        caret: Caret,
    },
}

/// What the window reports. A click selects and never commits; a page
/// turn is the panel's page up / page down.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum Event {
    Click { position: u32 },
    Page { forward: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_show_round_trips_as_one_line() {
        let command = Command::Show {
            body: Show {
                cells: vec![Cell {
                    text: "好".into(),
                    annotation: Some("hó".into()),
                }],
                labels: vec!["1".into()],
                cursor: Some(0),
                vertical: false,
                has_prev: false,
                has_next: true,
                hanji_px: 18,
                roman_px: 14,
                hanji_leads: true,
                chrome: Chrome::default(),
                caret: Caret {
                    x: 10,
                    y: 20,
                    w: 4,
                    h: 16,
                },
            },
        };
        let line = serde_json::to_string(&command).expect("serialize");
        assert!(!line.contains('\n'));
        let back: Command = serde_json::from_str(&line).expect("parse");
        assert_eq!(back, command);
    }
}
