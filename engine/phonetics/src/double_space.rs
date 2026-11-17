//! The double-space period, the one copy iOS and Android ask
//! (`DoubleSpacePeriod`): a second space press soon after a plain one, behind
//! a letter, number or mark, replaces the first space with a period.

use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

/// A second space this soon after a plain space is a double space.
const DOUBLE_SPACE_WINDOW_MS: i64 = 300;

/// What a double space writes in place of its first space, or `None` when it
/// writes a plain space: the press must come under [`DOUBLE_SPACE_WINDOW_MS`]
/// after the plain one, and `text_before_caret` must end in one ASCII space
/// after a letter, number or combining mark (a code point, so a
/// supplementary-plane Hanji and POJ's U+0358 both count; punctuation, emoji
/// or a second space do not). Full width writes `。` with no space; half width
/// writes `. `.
pub(crate) fn double_space_period(
    text_before_caret: &str,
    millis_since_plain_space: i64,
    full_width: bool,
) -> Option<&'static str> {
    if !(0..DOUBLE_SPACE_WINDOW_MS).contains(&millis_since_plain_space) {
        return None;
    }
    let previous = text_before_caret.strip_suffix(' ')?.chars().next_back()?;
    match previous.general_category_group() {
        GeneralCategoryGroup::Letter
        | GeneralCategoryGroup::Number
        | GeneralCategoryGroup::Mark => Some(if full_width { "。" } else { ". " }),
        _ => None,
    }
}
