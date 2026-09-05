//! Pins the `.rc` rendering the two Windows build scripts share
//! (`windows/build-support/resource.rs`) on the host: a build script has no
//! test target of its own, and a wrong `LANGUAGE` split or string escape would
//! only surface as a mis-named input method on a Windows machine.

// 在 macOS 主機上鎖住 build script 的 .rc 產生邏輯(LANGID 拆解、字串跳脫、resource id)。

#[path = "../../../build-support/resource.rs"]
#[allow(dead_code)]
mod resource;

#[test]
fn one_string_table_per_language_with_the_langid_split_into_primary_and_sub() {
    let rendered = resource::string_tables(
        &[
            (0x0409, "TaigiKeyboard"),
            (0x0411, "台湾語キーボード"),
            (0x0404, "台語齒盤"),
        ],
        100,
    );
    assert_eq!(
        rendered,
        "STRINGTABLE LANGUAGE 0x09, 0x01\nBEGIN\n    100 \"TaigiKeyboard\"\nEND\n\
         STRINGTABLE LANGUAGE 0x11, 0x01\nBEGIN\n    100 \"台湾語キーボード\"\nEND\n\
         STRINGTABLE LANGUAGE 0x04, 0x01\nBEGIN\n    100 \"台語齒盤\"\nEND\n"
    );
}

#[test]
fn rc_string_escapes_quotes_and_backslashes() {
    let rendered = resource::string_tables(&[(0x0409, r#"a"b\c"#)], 7);
    assert!(rendered.contains(r#"    7 "a""b\\c""#), "{rendered}");
}

#[test]
fn the_generated_names_render_under_the_exported_id() {
    // `product_name_strings.rs` is `make i18n` output; every entry must reach
    // the table under the id `registration::profile_description` points at.
    let rendered = resource::product_name_string_tables();
    let id = resource::PRODUCT_NAME_STRING_ID;
    assert_eq!(id, 100, "registration.rs documents `@<dll>,-100`");
    for (langid, name) in resource::PRODUCT_NAMES {
        assert!(rendered.contains(&format!("0x{:02X}, 0x{:02X}", langid & 0x3FF, langid >> 10)));
        assert!(rendered.contains(&format!("    {id} \"{name}\"")));
    }
}
