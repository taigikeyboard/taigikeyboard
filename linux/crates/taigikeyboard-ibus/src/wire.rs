//! The IBus wire types an engine sends, hand-serialised to the GVariant
//! shapes ibus itself emits (roadmap L1): every `IBusSerializable` is a
//! tuple `(s type_name, a{sv} attachments, …fields)` (ibus
//! `src/ibusserializable.c:277`), and every nested serialisable rides inside
//! a `v`. Field ORDER is the compatibility contract ("The serialized order
//! should be kept", `src/ibusenginedesc.c`), so each builder pins its
//! signature in a test.
//!
//! Nearly everything goes engine → daemon. The one serialisable the
//! daemon sends that this engine decodes is `SetSurroundingText`'s text
//! ([`text_of`]).

use zbus::zvariant::{Array, Dict, Signature, Str, Structure, StructureBuilder, Value};

/// `IBUS_ATTR_TYPE_UNDERLINE` (ibus `src/ibusattribute.h:78`).
const ATTR_TYPE_UNDERLINE: u32 = 1;
/// `IBUS_ATTR_UNDERLINE_SINGLE` (`:96`).
const ATTR_UNDERLINE_SINGLE: u32 = 1;

/// `IBUS_ORIENTATION_HORIZONTAL` / `VERTICAL` (ibus `src/ibustypes.h:151`).
/// `Vertical` stays: it is a wire value. The candidate window draws the list,
/// so this process no longer asks the panel for a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal = 0,
    #[allow(dead_code)]
    Vertical = 1,
}

/// The empty `a{sv}` every serialisable starts with (the "attachments"
/// dictionary, which no engine of ours fills).
fn attachments() -> Value<'static> {
    Value::Dict(Dict::new(&Signature::Str, &Signature::Variant))
}

fn serializable(type_name: &'static str) -> StructureBuilder<'static> {
    StructureBuilder::new()
        .add_field(Str::from_static(type_name))
        .append_field(attachments())
}

fn boxed(value: Value<'static>) -> Value<'static> {
    Value::Value(Box::new(value))
}

fn variant_array(items: Vec<Value<'static>>) -> Value<'static> {
    let mut array = Array::new(&Signature::Variant);
    for item in items {
        array
            .append(boxed(item))
            .expect("an `av` accepts every boxed variant");
    }
    Value::Array(array)
}

fn built(builder: StructureBuilder<'static>) -> Structure<'static> {
    builder
        .build()
        .expect("a serialisable built from constants has a valid signature")
}

/// `IBusAttribute` = `("IBusAttribute", a{sv}, u type, u value, u start, u end)`
/// (ibus `src/ibusattribute.c`, `ibus_attribute_serialize`). Indices are in
/// characters (what `ibus_text` counts), start inclusive, end exclusive.
pub fn attribute(kind: u32, value: u32, start: u32, end: u32) -> Value<'static> {
    Value::Structure(built(
        serializable("IBusAttribute")
            .add_field(kind)
            .add_field(value)
            .add_field(start)
            .add_field(end),
    ))
}

/// `IBusAttrList` = `("IBusAttrList", a{sv}, av)` (`src/ibusattrlist.c`).
pub fn attr_list(attributes: Vec<Value<'static>>) -> Value<'static> {
    Value::Structure(built(
        serializable("IBusAttrList").append_field(variant_array(attributes)),
    ))
}

/// `IBusText` = `("IBusText", a{sv}, s text, v attrs)` (`src/ibustext.c`,
/// `ibus_text_serialize`; an absent list is serialised as an empty one).
pub fn text(text: &str, attributes: Vec<Value<'static>>) -> Value<'static> {
    Value::Structure(built(
        serializable("IBusText")
            .add_field(text.to_owned())
            .append_field(boxed(attr_list(attributes))),
    ))
}

/// A plain text with no attributes.
/// The string of an `IBusText` the daemon sent: `(s "IBusText", a{sv},
/// s text, v attrs)`, possibly still inside its `v`.
pub fn text_of<'a>(value: &'a Value<'_>) -> Option<&'a str> {
    match value {
        Value::Value(inner) => text_of(inner),
        Value::Structure(structure) => match structure.fields() {
            [Value::Str(type_name), _, Value::Str(text), ..]
                if type_name.as_str() == "IBusText" =>
            {
                Some(text.as_str())
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn plain_text(text: &str) -> Value<'static> {
    self::text(text, Vec::new())
}

/// The preedit: the whole string underlined once, as the composition is
/// marked on macOS and Windows.
pub fn preedit_text(text: &str) -> Value<'static> {
    let length = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    self::text(
        text,
        vec![attribute(
            ATTR_TYPE_UNDERLINE,
            ATTR_UNDERLINE_SINGLE,
            0,
            length,
        )],
    )
}

/// `IBusLookupTable` = `("IBusLookupTable", a{sv}, u page_size, u cursor_pos,
/// b cursor_visible, b round, i orientation, av candidates, av labels)`
/// (`src/ibuslookuptable.c`, `ibus_lookup_table_serialize`).
pub struct LookupTable<'a> {
    pub page_size: u32,
    pub cursor_pos: u32,
    pub cursor_visible: bool,
    /// Whether the last page wraps to the first.
    pub round: bool,
    pub orientation: Orientation,
    pub candidates: &'a [String],
    /// One label per page POSITION (the panel indexes them by row on the
    /// page), so `page_size` of them.
    pub labels: &'a [String],
}

impl LookupTable<'_> {
    pub fn to_value(&self) -> Value<'static> {
        Value::Structure(built(
            serializable("IBusLookupTable")
                .add_field(self.page_size)
                .add_field(self.cursor_pos)
                .add_field(self.cursor_visible)
                .add_field(self.round)
                .add_field(self.orientation as i32)
                .append_field(variant_array(
                    self.candidates.iter().map(|c| plain_text(c)).collect(),
                ))
                .append_field(variant_array(
                    self.labels.iter().map(|l| plain_text(l)).collect(),
                )),
        ))
    }
}

/// `IBusPropType` (ibus `src/ibusproperty.h:79-88`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropType {
    Normal = 0,
    Menu = 3,
    Separator = 4,
}

/// `IBusProperty` = `("IBusProperty", a{sv}, s key, u type, v label, s icon,
/// v tooltip, b sensitive, b visible, u state, v sub_props, v symbol)`
/// (`src/ibusproperty.c`, `ibus_property_serialize`). `label`, `tooltip`
/// and `symbol` are `IBusText`s; `sub_props` is an `IBusPropList`.
pub struct Property<'a> {
    pub key: &'a str,
    pub kind: PropType,
    pub label: &'a str,
    /// The accelerator column: ibus draws the tooltip on hover, which is
    /// where the recorded chord goes.
    pub tooltip: &'a str,
    /// An icon theme name or a file path; "" for none. The panel's
    /// indicator icon for a menu root.
    pub icon: &'a str,
    /// The panel's indicator text for a menu root.
    pub symbol: &'a str,
    pub sub_props: Vec<Value<'static>>,
}

impl Property<'_> {
    pub fn to_value(&self) -> Value<'static> {
        Value::Structure(built(
            serializable("IBusProperty")
                .add_field(self.key.to_owned())
                .add_field(self.kind as u32)
                .append_field(boxed(plain_text(self.label)))
                .add_field(self.icon.to_owned())
                .append_field(boxed(plain_text(self.tooltip)))
                .add_field(true)
                .add_field(true)
                .add_field(0u32)
                .append_field(boxed(prop_list(self.sub_props.clone())))
                .append_field(boxed(plain_text(self.symbol))),
        ))
    }
}

/// `IBusPropList` = `("IBusPropList", a{sv}, av)` (`src/ibusproplist.c`).
pub fn prop_list(properties: Vec<Value<'static>>) -> Value<'static> {
    Value::Structure(built(
        serializable("IBusPropList").append_field(variant_array(properties)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_and_prop_list_have_the_ibus_signatures() {
        // trace: `ibus_property_serialize` appends s u v s v b b u v v after
        // the serialisable prefix.
        let row = Property {
            key: "settings",
            kind: PropType::Normal,
            label: "設定",
            tooltip: "Ctrl+Alt+S",
            icon: "",
            symbol: "",
            sub_props: Vec::new(),
        }
        .to_value();
        assert_eq!(signature_of(&row), "(sa{sv}suvsvbbuvv)");
        let root = Property {
            key: "taigikeyboard",
            kind: PropType::Menu,
            label: "台語齒盤",
            tooltip: "",
            icon: "taigikeyboard-tl-hanji",
            symbol: "台",
            sub_props: vec![row],
        }
        .to_value();
        assert_eq!(signature_of(&root), "(sa{sv}suvsvbbuvv)");
        // trace: fields = type name, attachments, key, type, label, icon, …
        let Value::Structure(structure) = &root else {
            panic!("a property is a structure")
        };
        assert_eq!(structure.fields()[5], Value::from("taigikeyboard-tl-hanji"));
        assert_eq!(signature_of(&prop_list(vec![root])), "(sa{sv}av)");
    }

    fn signature_of(value: &Value<'_>) -> String {
        value.value_signature().to_string()
    }

    #[test]
    fn a_serialisable_starts_with_its_type_name_and_an_empty_attachment_dict() {
        // trace: `ibus_serializable_real_serialize` adds "s" then "a{sv}".
        let value = plain_text("x");
        assert_eq!(signature_of(&value), "(sa{sv}sv)");
        let Value::Structure(structure) = value else {
            panic!("a text is a structure")
        };
        assert_eq!(structure.fields()[0], Value::from("IBusText"));
        assert_eq!(signature_of(&structure.fields()[1]), "a{sv}");
        assert_eq!(structure.fields()[2], Value::from("x"));
    }

    #[test]
    fn an_ibus_text_reads_back_bare_or_boxed() {
        let sent = plain_text("guá 2");
        assert_eq!(text_of(&sent), Some("guá 2"));
        assert_eq!(text_of(&boxed(sent.try_clone().unwrap())), Some("guá 2"));
        assert_eq!(text_of(&Value::from("guá")), None);
    }

    #[test]
    fn text_attribute_and_attr_list_have_the_ibus_signatures() {
        assert_eq!(signature_of(&attribute(1, 1, 0, 3)), "(sa{sv}uuuu)");
        assert_eq!(signature_of(&attr_list(vec![])), "(sa{sv}av)");
        assert_eq!(signature_of(&preedit_text("tâi")), "(sa{sv}sv)");
    }

    #[test]
    fn the_preedit_underline_spans_the_text_in_characters_not_bytes() {
        // trace: "tâi" is 3 characters, 4 bytes; ibus counts characters.
        let Value::Structure(text) = preedit_text("tâi") else {
            panic!()
        };
        let Value::Value(attrs) = &text.fields()[3] else {
            panic!("attrs ride in a variant")
        };
        let Value::Structure(list) = attrs.as_ref() else {
            panic!()
        };
        let Value::Array(items) = &list.fields()[2] else {
            panic!()
        };
        let Value::Value(first) = &items.inner()[0] else {
            panic!()
        };
        let Value::Structure(attribute) = first.as_ref() else {
            panic!()
        };
        assert_eq!(
            &attribute.fields()[2..],
            &[Value::U32(1), Value::U32(1), Value::U32(0), Value::U32(3)]
        );
    }

    #[test]
    fn the_lookup_table_signature_matches_ibus_lookup_table_serialize() {
        let candidates = vec!["台".to_owned(), "tâi".to_owned()];
        let labels = vec!["q".to_owned(), "w".to_owned()];
        let table = LookupTable {
            page_size: 9,
            cursor_pos: 0,
            cursor_visible: true,
            round: false,
            orientation: Orientation::Horizontal,
            candidates: &candidates,
            labels: &labels,
        };
        assert_eq!(signature_of(&table.to_value()), "(sa{sv}uubbiavav)");
        let Value::Structure(structure) = table.to_value() else {
            panic!()
        };
        assert_eq!(structure.fields()[6], Value::I32(0));
        let Value::Array(items) = &structure.fields()[7] else {
            panic!()
        };
        assert_eq!(items.len(), 2);
    }
}
