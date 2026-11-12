//! The typeface a family name actually draws with.
//!
//! cosmic-text keeps a named family only when some face of that family has
//! the requested weight exactly. A Pango family whose regular face is not
//! weight 400, or whose stored name differs by case or spaces, falls through
//! to the script fallback and the setting looks unchanged. Resolve the name
//! to a face the database has, then ask for that face's own weight, style
//! and stretch.

use cosmic_text::{Stretch, Style, Weight};
use fontdb::Database;

#[derive(Clone, Debug)]
pub struct ResolvedFace {
    pub family: String,
    pub weight: Weight,
    pub style: Style,
    pub stretch: Stretch,
}

impl ResolvedFace {
    fn system() -> Self {
        Self {
            family: String::new(),
            weight: Weight::NORMAL,
            style: Style::Normal,
            stretch: Stretch::Normal,
        }
    }
}

pub fn resolve_face(db: &Database, requested: &str) -> ResolvedFace {
    let requested = requested.trim();
    if requested.is_empty() {
        return ResolvedFace::system();
    }
    let (family, want) = split_request(db, requested);
    let folded = fold(&family);
    let mut best: Option<Rank> = None;
    for face in db.faces() {
        let Some(name) = matching_name(face, &family, &folded) else {
            continue;
        };
        let rank = Rank {
            weight_diff: face
                .weight
                .0
                .abs_diff(want.weight.unwrap_or(Weight::NORMAL).0),
            style_rank: u8::from(face.style != want.style.unwrap_or(Style::Normal)),
            stretch_rank: u8::from(face.stretch != Stretch::Normal),
            name: name.to_owned(),
            weight: face.weight,
            style: face.style,
            stretch: face.stretch,
        };
        let replace = best.as_ref().is_none_or(|current| rank.precedes(current));
        if replace {
            best = Some(rank);
        }
    }
    match best {
        Some(rank) => ResolvedFace {
            family: rank.name,
            weight: rank.weight,
            style: rank.style,
            stretch: rank.stretch,
        },
        None => ResolvedFace {
            family: requested.to_owned(),
            ..ResolvedFace::system()
        },
    }
}

struct Rank {
    weight_diff: u16,
    style_rank: u8,
    stretch_rank: u8,
    name: String,
    weight: Weight,
    style: Style,
    stretch: Stretch,
}

impl Rank {
    fn precedes(&self, other: &Self) -> bool {
        (self.weight_diff, self.style_rank, self.stretch_rank)
            < (other.weight_diff, other.style_rank, other.stretch_rank)
    }
}

fn matching_name<'a>(face: &'a fontdb::FaceInfo, requested: &str, folded: &str) -> Option<&'a str> {
    if let Some((name, _)) = face
        .families
        .iter()
        .find(|(name, _)| name == requested || (!folded.is_empty() && fold(name) == folded))
    {
        return Some(name);
    }
    let postscript = face.post_script_name.as_str();
    if postscript.eq_ignore_ascii_case(requested)
        || (!folded.is_empty() && fold(postscript) == folded)
    {
        face.families.first().map(|(name, _)| name.as_str())
    } else {
        None
    }
}

struct WantedFace {
    weight: Option<Weight>,
    style: Option<Style>,
}

/// A stored face is a family, or that family plus style words (`Bold`,
/// `Italic`). Style words are peeled off first, so `DejaVu Sans Regular`
/// is the regular face of `DejaVu Sans` and not a family whose name ends
/// in Regular. A family that really is named `… Bold` is used whole when
/// the stem is not itself a family.
fn split_request(db: &Database, requested: &str) -> (String, WantedFace) {
    let (stem, want) = peel_style(requested);
    if !stem.is_empty() {
        if let Some(family) = family_named(db, &stem) {
            return (family, want);
        }
    }
    if let Some(family) = family_named(db, requested) {
        return (
            family,
            WantedFace {
                weight: None,
                style: None,
            },
        );
    }
    (
        requested.to_owned(),
        WantedFace {
            weight: None,
            style: None,
        },
    )
}

fn peel_style(requested: &str) -> (String, WantedFace) {
    let mut words: Vec<&str> = requested.split_whitespace().collect();
    let mut weight = None;
    let mut style = None;
    while let Some(last) = words.last().copied() {
        let (next_weight, next_style) = style_words(last);
        if next_weight.is_none() && next_style.is_none() {
            break;
        }
        words.pop();
        weight = next_weight.or(weight);
        style = next_style.or(style);
    }
    (words.join(" "), WantedFace { weight, style })
}

fn family_named(db: &Database, requested: &str) -> Option<String> {
    let folded = fold(requested);
    if folded.is_empty() {
        return None;
    }
    db.faces().find_map(|face| {
        face.families
            .iter()
            .find_map(|(name, _)| (fold(name) == folded).then(|| name.clone()))
    })
}

fn style_words(rest: &str) -> (Option<Weight>, Option<Style>) {
    let mut weight = None;
    let mut style = None;
    for token in rest.split(|scalar: char| scalar.is_whitespace() || scalar == ',' || scalar == '-')
    {
        match fold(token).as_str() {
            "thin" | "hairline" => weight = Some(Weight(100)),
            "extralight" | "ultralight" => weight = Some(Weight(200)),
            "light" => weight = Some(Weight(300)),
            "regular" | "normal" | "book" | "roman" => weight = Some(Weight::NORMAL),
            "medium" => weight = Some(Weight(500)),
            "semibold" | "demibold" => weight = Some(Weight(600)),
            "bold" => weight = Some(Weight::BOLD),
            "extrabold" | "ultrabold" => weight = Some(Weight(800)),
            "black" | "heavy" => weight = Some(Weight(900)),
            "italic" => style = Some(Style::Italic),
            "oblique" => style = Some(Style::Oblique),
            _ => {}
        }
    }
    (weight, style)
}

fn fold(name: &str) -> String {
    name.chars()
        .filter(|scalar| !scalar.is_whitespace() && *scalar != '-' && *scalar != '_')
        .flat_map(|scalar| scalar.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
    use std::path::Path;

    #[test]
    fn a_bold_only_family_is_the_face_that_shapes() {
        let sans = Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf");
        let serif = Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf");
        if !sans.exists() || !serif.exists() {
            return;
        }
        let mut db = Database::new();
        db.load_font_file(sans).expect("sans");
        db.load_font_file(serif).expect("serif");
        let resolved = resolve_face(&db, "dejavu sans");
        assert_eq!(resolved.family, "DejaVu Sans");
        assert_eq!(resolved.weight, Weight::BOLD);
        let mut system = FontSystem::new_with_locale_and_db("en-US".into(), db);
        let attrs = Attrs::new()
            .family(Family::Name(&resolved.family))
            .weight(resolved.weight)
            .style(resolved.style)
            .stretch(resolved.stretch);
        let mut buffer = Buffer::new(&mut system, Metrics::new(20.0, 20.0));
        buffer.set_size(&mut system, None, None);
        buffer.set_text(&mut system, "A", &attrs, Shaping::Advanced);
        let mut saw = false;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let face = system.db().face(glyph.font_id).expect("face");
                assert!(
                    face.families.iter().any(|(name, _)| name == "DejaVu Sans"),
                    "shaped {:?}",
                    face.families
                );
                saw = true;
            }
        }
        assert!(saw);
    }

    #[test]
    fn a_style_suffix_selects_that_face_of_the_family() {
        let regular = Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
        let bold = Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf");
        if !regular.exists() || !bold.exists() {
            return;
        }
        let mut db = Database::new();
        db.load_font_file(regular).expect("regular");
        db.load_font_file(bold).expect("bold");
        assert_eq!(resolve_face(&db, "DejaVu Sans").weight, Weight::NORMAL);
        assert_eq!(
            resolve_face(&db, "DejaVu Sans Regular").weight,
            Weight::NORMAL
        );
        assert_eq!(resolve_face(&db, "DejaVu Sans Bold").weight, Weight::BOLD);
    }

    #[test]
    fn a_system_cjk_family_shapes_hanji_when_installed() {
        let mut system = FontSystem::new();
        let requested = "Noto Sans CJK TC";
        let present = system
            .db()
            .faces()
            .any(|face| face.families.iter().any(|(name, _)| name == requested));
        if !present {
            return;
        }
        let resolved = resolve_face(system.db(), requested);
        assert_eq!(resolved.family, requested);
        let attrs = Attrs::new()
            .family(Family::Name(&resolved.family))
            .weight(resolved.weight)
            .style(resolved.style)
            .stretch(resolved.stretch);
        let mut buffer = Buffer::new(&mut system, Metrics::new(32.0, 32.0));
        buffer.set_size(&mut system, None, None);
        buffer.set_text(&mut system, "好", &attrs, Shaping::Advanced);
        let mut saw = false;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let face = system.db().face(glyph.font_id).expect("face");
                assert!(
                    face.families.iter().any(|(name, _)| name == requested),
                    "shaped {:?}",
                    face.families
                );
                saw = true;
            }
        }
        assert!(saw);
    }
}
