//! Every pane's view must PLAN, and every sidebar switch must APPLY — the
//! regression net for the whole window.
//!
//! The reactor refuses a tree whose shape it cannot realize and answers
//! `PumpError::StructureUnsupported`; what that costs is written out once,
//! on `cards::frame`. Two such trees shipped in W17 because nothing ever
//! mounted these pages, and the crash reaches the user with no message.
//!
//! `RecordingRuntime` is the reactor's headless host, so planning runs with
//! no WinUI runtime and no desktop — exactly the layer both defects were
//! in. **Scope: each pane in its LAUNCH state, plus one switch between
//! every two sidebar panes.** A subtree that only
//! appears once the user has done something — the busy overlay, the entry
//! dialog, a search result row — returns `View::empty()` here and is not
//! covered; so is everything below planning (the Windows App SDK ABI, COM
//! apartments, real layout, theme, focus). Those stay device dogfood.

use super::cards;
use super::list_selection::recorded::{insertion_parents, selections};
use super::window::{SettingsWindow, SettingsWindowInput};
use taigi_desktop_core::settings::{keys, SettingChoice, SettingsPane};
use taigi_desktop_storage::{LiveSettings, SettingsFileStore};
use tempfile::TempDir;
use windows_reactor::{
    Command, EventId, EventPayload, NodeId, PropertyId, PropertyValue, Pump, QueuedEvent,
    RecordingRuntime, SelectionChange, SlotId, View,
};

/// Far enough ahead that `update_schedule::is_due` says no: a writable
/// launch runs the overdue update check in `create`, and a test must not
/// reach the network.
const NEVER_DUE_MS: i64 = i64::MAX;

/// One settings directory for every mount below: they all want the same
/// stamped defaults, and none of them writes.
fn stamped_directory() -> TempDir {
    let directory = tempfile::tempdir().expect("a temp directory");
    SettingsFileStore::new(directory.path())
        .update(|document| document.set_i64(&keys::UPDATE_NEXT_CHECK_MS, NEVER_DUE_MS))
        .expect("stamp the update check away");
    directory
}

/// Mounts the whole window on `pane` against the headless runtime.
///
/// The user-data stores stay closed — `user_data::open_at_launch`'s job is
/// the launch's, and a test that only plans a view tree must not run its
/// migrations. The engine answers a page's first load with a refusal
/// (nothing is open in this process), which the tree under test shows as
/// the alert a failed load shows.
fn plan(directory: &TempDir, pane: SettingsPane, is_read_only: bool) -> Result<(), String> {
    planned(directory, pane, is_read_only).map(|_| ())
}

/// The same mount, keeping the runtime so a test can read what was applied.
fn planned(
    directory: &TempDir,
    pane: SettingsPane,
    is_read_only: bool,
) -> Result<Pump<RecordingRuntime>, String> {
    let input = SettingsWindowInput::new(
        LiveSettings::new(SettingsFileStore::new(directory.path())),
        is_read_only,
        pane,
        false,
    );
    let mut pump = Pump::new(RecordingRuntime::default());
    pump.mount_view(View::component::<SettingsWindow>(input))
        .map_err(|error| format!("{error:?}"))?;
    Ok(pump)
}

/// No pane's LAUNCH plan may select a row in the batch that inserts it.
///
/// Reactor plans a node's properties before its children, so a
/// `ListViewSelectedIndex` sent in the same batch as the items reaches a list
/// that is still one render behind it. XAML answers `E_INVALIDARG` for an
/// index it does not number, and reactor turns a failed native command into
/// `std::process::abort()` — the settings window died this way the first time
/// a user added a custom typeface (2026-09-11). `winui::list_selection` is
/// what holds the index back, and its own reactor tests drive the handover
/// across renders; this is the launch-state half — conservative on purpose
/// (a batch may insert OTHER rows harmlessly), and blind to everything a user
/// has to do something to reach.
#[test]
fn no_pane_selects_a_row_in_the_render_that_inserts_it() {
    let directory = stamped_directory();
    for pane in SettingsPane::ALL {
        let pump = planned(&directory, *pane, false).expect("the pane plans");
        for (batch_index, batch) in pump.runtime().commands().iter().enumerate() {
            let parents = insertion_parents(batch);
            for (node, index) in selections(batch) {
                assert!(
                    !parents.contains(&node),
                    "pane {} batch {batch_index}: selects index {index} of a list \
                     the same batch fills",
                    pane.raw(),
                );
            }
        }
    }
}

#[test]
fn every_pane_plans_writable_and_read_only() {
    let directory = stamped_directory();
    // Read-only is its own tree, not a cosmetic difference: the window
    // draws a banner and greys the controls (`SettingsWriter`, roadmap W2).
    for is_read_only in [false, true] {
        for pane in SettingsPane::ALL {
            assert_eq!(
                plan(&directory, *pane, is_read_only),
                Ok(()),
                "pane {} does not plan (read_only={is_read_only})",
                pane.raw()
            );
        }
    }
}

/// Every sidebar switch must APPLY, not just plan: the launch tests above
/// mount one pane, and a defect that lives in the diff between two panes
/// never shows there. Shortcuts ⇄ Dictionary Sources aborted the window on
/// a real `InsertChild` E_BOUNDS (2026-10-08) because the page slot was
/// unkeyed — `RecordingRuntime` bounds-checks child inserts the way XAML
/// does, so the same switch fails here.
#[test]
fn every_sidebar_switch_applies() {
    let directory = stamped_directory();
    let mut failures = Vec::new();
    for from in SettingsPane::SIDEBAR {
        for to in SettingsPane::SIDEBAR {
            if from == to {
                continue;
            }
            let mut pump = planned(&directory, from, false).expect("the pane plans");
            let navigation = switch_pane(&mut pump, to);
            let header_before = header_text(&pump, navigation);
            let events = pump.dispatch_events();
            let turns = pump.dispatch_components(16);
            // A switch the window ignored would apply nothing and pass; the
            // header title is what changes with the pane.
            let is_switched = header_text(&pump, navigation) != header_before;
            if events.is_err() || turns.is_err() || pump.poisoned() || !is_switched {
                failures.push(format!(
                    "{} -> {}: {events:?} {turns:?} switched={is_switched}",
                    from.raw(),
                    to.raw()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "pane switches that fail:\n{}",
        failures.join("\n")
    );
}

fn find_navigation_view(pump: &Pump<RecordingRuntime>) -> NodeId {
    pump.runtime()
        .commands()
        .iter()
        .flatten()
        .find_map(|command| match command {
            // `MountedKind` is private to reactor; its Debug name is the
            // only handle on the kind a test has.
            Command::Create { node, kind }
                if format!("{kind:?}") == "NavigationView"
                    && pump.runtime().node(*node).is_some() =>
            {
                Some(*node)
            }
            _ => None,
        })
        .expect("the window mounts a NavigationView")
}

/// Queues the sidebar selection a click on `to` delivers; returns the
/// NavigationView.
fn switch_pane(pump: &mut Pump<RecordingRuntime>, to: SettingsPane) -> NodeId {
    let navigation = find_navigation_view(pump);
    let revision = pump
        .event_revision(navigation, EventId::NavigationViewSelectionChanged)
        .expect("the sidebar listens for selection");
    let items = pump
        .runtime()
        .node(navigation)
        .expect("the NavigationView node")
        .slot_children(SlotId::NavigationViewMenuItems)
        .to_vec();
    let position = SettingsPane::SIDEBAR
        .iter()
        .position(|pane| *pane == to)
        .expect("a sidebar pane");
    pump.queue_event(QueuedEvent::new(
        navigation,
        EventId::NavigationViewSelectionChanged,
        revision,
        EventPayload::SelectionChange(SelectionChange {
            item: Some(items[position]),
            tag: Some(to.raw().into()),
        }),
    ));
    navigation
}

/// The title the NavigationView header shows.
fn header_text(pump: &Pump<RecordingRuntime>, navigation: NodeId) -> Option<String> {
    let runtime = pump.runtime();
    let header = runtime
        .node(navigation)?
        .slot(SlotId::NavigationViewHeader)?;
    match runtime.node(header)?.property(PropertyId::TextBlockText)? {
        PropertyValue::Str(text) => Some(text.clone()),
        _ => None,
    }
}

fn collect_content_nodes(pump: &Pump<RecordingRuntime>) -> Vec<NodeId> {
    let navigation = find_navigation_view(pump);
    let runtime = pump.runtime();
    let Some(content) = runtime
        .node(navigation)
        .and_then(|node| node.slot(SlotId::NavigationViewContent))
    else {
        return Vec::new();
    };
    let mut visited = Vec::new();
    let mut stack = vec![content];
    while let Some(node_id) = stack.pop() {
        let Some(node) = runtime.node(node_id) else {
            continue;
        };
        visited.push(node_id);
        for &child in node.children() {
            stack.push(child);
        }
    }
    visited
}

fn subtree_contains_text(runtime: &RecordingRuntime, root: NodeId, text: &str) -> bool {
    let mut stack = vec![root];
    while let Some(node_id) = stack.pop() {
        let Some(node) = runtime.node(node_id) else {
            continue;
        };
        if node
            .kind()
            .is_some_and(|kind| format!("{kind:?}") == "TextBlock")
            && matches!(
                node.property(PropertyId::TextBlockText),
                Some(PropertyValue::Str(value)) if value == text
            )
        {
            return true;
        }
        for &child in node.children() {
            stack.push(child);
        }
    }
    false
}

fn text_blocks_matching(
    pump: &Pump<RecordingRuntime>,
    text: &str,
) -> Vec<(NodeId, Option<f64>, Option<f64>)> {
    let runtime = pump.runtime();
    collect_content_nodes(pump)
        .into_iter()
        .filter_map(|node_id| {
            let node = runtime.node(node_id)?;
            if !node
                .kind()
                .is_some_and(|kind| format!("{kind:?}") == "TextBlock")
            {
                return None;
            }
            match node.property(PropertyId::TextBlockText) {
                Some(PropertyValue::Str(value)) if value == text => {}
                _ => return None,
            }
            let opacity = match node.property(PropertyId::Opacity) {
                Some(PropertyValue::F64(value)) => Some(*value),
                _ => None,
            };
            let font_size = match node.property(PropertyId::TextBlockFontSize) {
                Some(PropertyValue::F64(value)) => Some(*value),
                _ => None,
            };
            Some((node_id, opacity, font_size))
        })
        .collect()
}

fn find_row_combo_box(pump: &Pump<RecordingRuntime>, header: &str) -> NodeId {
    let runtime = pump.runtime();
    for node_id in collect_content_nodes(pump) {
        let Some(node) = runtime.node(node_id) else {
            continue;
        };
        if !node
            .kind()
            .is_some_and(|kind| format!("{kind:?}") == "Grid")
        {
            continue;
        }
        let children = node.children();
        if children.len() != 2 || !subtree_contains_text(runtime, children[0], header) {
            continue;
        }
        let mut stack = vec![children[1]];
        while let Some(candidate) = stack.pop() {
            let Some(child_node) = runtime.node(candidate) else {
                continue;
            };
            if child_node
                .kind()
                .is_some_and(|kind| format!("{kind:?}") == "ComboBox")
            {
                return candidate;
            }
            for &nested in child_node.children() {
                stack.push(nested);
            }
        }
    }
    panic!("no ComboBox row found with header {header:?}");
}

fn directory_with_settings(
    mutate: impl FnOnce(&mut taigi_desktop_core::settings::SettingsDocument),
) -> TempDir {
    let directory = tempfile::tempdir().expect("a temp directory");
    SettingsFileStore::new(directory.path())
        .update(|document| {
            document.set_i64(&keys::UPDATE_NEXT_CHECK_MS, NEVER_DUE_MS);
            mutate(document);
        })
        .expect("stamp test settings");
    directory
}

#[test]
fn row_caption_prefers_selected_option_and_falls_back_to_row_key() {
    use crate::winui::pages::{row_caption, RowLabel};
    use taigi_desktop_core::keys::ToneInputScheme;
    use taigi_desktop_core::settings::SyllableSeparator;
    use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};

    for language in DisplayLanguage::PICKER.into_iter().skip(1) {
        let strings = StringResolver::new(language);
        let numeric = row_caption(
            &strings,
            StringKey::SettingsToneInputScheme,
            Some(ToneInputScheme::Standard.label_key()),
        );
        let telex = row_caption(
            &strings,
            StringKey::SettingsToneInputScheme,
            Some(ToneInputScheme::Telex.label_key()),
        );
        assert_eq!(
            numeric,
            Some(strings.resolve(StringKey::SettingsToneSchemeStandardInfo)),
            "{language:?}"
        );
        assert_eq!(
            telex,
            Some(strings.resolve(StringKey::SettingsToneSchemeTelexInfo)),
            "{language:?}"
        );
        assert_ne!(numeric, telex, "{language:?}");

        let separator = row_caption(
            &strings,
            StringKey::SettingsSyllableSeparator,
            Some(SyllableSeparator::Hyphen.label_key()),
        );
        assert_eq!(
            separator,
            Some(strings.resolve(StringKey::SettingsSyllableSeparatorInfo)),
            "{language:?}"
        );

        assert_eq!(
            row_caption(&strings, StringKey::DictionaryCustomDictEnabled, None),
            None,
            "{language:?}"
        );

        let telex_row = RowLabel::with_option(
            &strings,
            StringKey::SettingsToneInputScheme,
            Some(ToneInputScheme::Telex.label_key()),
        );
        assert_eq!(
            telex_row.header,
            strings.resolve(StringKey::SettingsToneInputScheme),
            "{language:?}"
        );
        assert_eq!(telex_row.description, telex, "{language:?}");

        let plain_row = RowLabel::plain(&strings, StringKey::SettingsAutoSpace);
        assert_eq!(
            plain_row.header,
            strings.resolve(StringKey::SettingsAutoSpace),
            "{language:?}"
        );
        assert_eq!(plain_row.description, None, "{language:?}");
    }
}

#[test]
fn general_pane_renders_expected_captions_across_production_languages() {
    use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};

    let visible_defaults = [
        StringKey::SettingsToneSchemeStandardInfo,
        StringKey::SettingsHanLoMixedInfo,
        StringKey::DesktopLiteralRomanCandidateInfo,
        StringKey::SettingsOutputBothScriptsInfo,
        StringKey::SettingsKhinsiannInHanjiInfo,
        StringKey::SettingsSyllableSeparatorInfo,
        StringKey::SettingsAutoSpaceInfo,
    ];

    for language in DisplayLanguage::PICKER.into_iter().skip(1) {
        let directory = directory_with_settings(|document| {
            document.set_string(&keys::DISPLAY_LANGUAGE, language.tag());
        });
        let pump = planned(&directory, SettingsPane::General, false).expect("the pane plans");
        let strings = StringResolver::new(language);

        for caption_key in visible_defaults {
            let text = strings.resolve(caption_key);
            let blocks = text_blocks_matching(&pump, text);
            assert_eq!(
                blocks.len(),
                1,
                "{language:?} expected 1 caption TextBlock for {caption_key:?}, found {}",
                blocks.len()
            );
            assert_eq!(
                blocks[0].1,
                Some(cards::SECONDARY_OPACITY),
                "{language:?} {caption_key:?} opacity"
            );
            assert_eq!(
                blocks[0].2,
                Some(cards::CAPTION_FONT_SIZE),
                "{language:?} {caption_key:?} font size"
            );
        }

        let telex_text = strings.resolve(StringKey::SettingsToneSchemeTelexInfo);
        assert!(
            text_blocks_matching(&pump, telex_text).is_empty(),
            "{language:?} unexpectedly rendered Telex caption under default Standard scheme"
        );
    }
}

#[test]
fn tone_keys_picker_swaps_caption_live_between_standard_and_telex() {
    use taigi_desktop_core::keys::ToneInputScheme;
    use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};

    assert_eq!(ToneInputScheme::ALL[0], ToneInputScheme::Standard);
    assert_eq!(ToneInputScheme::ALL[1], ToneInputScheme::Telex);

    let directory = directory_with_settings(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::English.tag());
    });
    let mut pump = planned(&directory, SettingsPane::General, false).expect("the pane plans");
    let strings = StringResolver::new(DisplayLanguage::English);

    let standard_caption = strings.resolve(StringKey::SettingsToneSchemeStandardInfo);
    let telex_caption = strings.resolve(StringKey::SettingsToneSchemeTelexInfo);
    assert_eq!(text_blocks_matching(&pump, standard_caption).len(), 1);
    assert!(text_blocks_matching(&pump, telex_caption).is_empty());

    let combo = find_row_combo_box(&pump, strings.resolve(StringKey::SettingsToneInputScheme));
    let revision = pump
        .event_revision(combo, EventId::ComboBoxSelectionChanged)
        .expect("ComboBox listens for selection");
    pump.queue_event(QueuedEvent::new(
        combo,
        EventId::ComboBoxSelectionChanged,
        revision,
        EventPayload::SelectionIndex(Some(1)),
    ));
    assert_eq!(pump.dispatch_events(), Ok(1));
    pump.dispatch_components(16).expect("components dispatch");

    assert!(text_blocks_matching(&pump, standard_caption).is_empty());
    assert_eq!(text_blocks_matching(&pump, telex_caption).len(), 1);

    let revision = pump
        .event_revision(combo, EventId::ComboBoxSelectionChanged)
        .expect("ComboBox listens for selection");
    pump.queue_event(QueuedEvent::new(
        combo,
        EventId::ComboBoxSelectionChanged,
        revision,
        EventPayload::SelectionIndex(Some(0)),
    ));
    assert_eq!(pump.dispatch_events(), Ok(1));
    pump.dispatch_components(16).expect("components dispatch");

    assert_eq!(text_blocks_matching(&pump, standard_caption).len(), 1);
    assert!(text_blocks_matching(&pump, telex_caption).is_empty());
}

#[test]
fn disabled_rows_dim_both_header_and_caption() {
    use taigi_desktop_core::settings::CandidateDisplayMode;
    use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};

    let directory = directory_with_settings(|document| {
        document.set_string(&keys::DISPLAY_LANGUAGE, DisplayLanguage::English.tag());
        document.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::RomanOnly,
        );
    });
    let pump = planned(&directory, SettingsPane::General, false).expect("the pane plans");
    let strings = StringResolver::new(DisplayLanguage::English);

    let disabled_captioned = [
        (
            StringKey::SettingsHanLoMixed,
            StringKey::SettingsHanLoMixedInfo,
        ),
        (
            StringKey::SettingsOutputBothScripts,
            StringKey::SettingsOutputBothScriptsInfo,
        ),
        (
            StringKey::SettingsKhinsiannInHanji,
            StringKey::SettingsKhinsiannInHanjiInfo,
        ),
    ];
    for (header_key, caption_key) in disabled_captioned {
        let headers = text_blocks_matching(&pump, strings.resolve(header_key));
        let captions = text_blocks_matching(&pump, strings.resolve(caption_key));
        assert_eq!(headers.len(), 1, "{header_key:?}");
        assert_eq!(captions.len(), 1, "{caption_key:?}");
        assert_eq!(
            headers[0].1,
            Some(cards::DISABLED_LABEL_OPACITY),
            "disabled header {header_key:?}"
        );
        assert_eq!(
            captions[0].1,
            Some(cards::DISABLED_LABEL_OPACITY),
            "disabled caption {caption_key:?}"
        );
    }

    let output_script_headers =
        text_blocks_matching(&pump, strings.resolve(StringKey::SettingsOutputScript));
    assert_eq!(output_script_headers.len(), 1);
    assert_eq!(
        output_script_headers[0].1,
        Some(cards::DISABLED_LABEL_OPACITY)
    );

    let enabled_captioned = [
        (
            StringKey::SettingsLiteralRomanCandidate,
            StringKey::DesktopLiteralRomanCandidateInfo,
        ),
        (
            StringKey::SettingsSyllableSeparator,
            StringKey::SettingsSyllableSeparatorInfo,
        ),
        (
            StringKey::SettingsAutoSpace,
            StringKey::SettingsAutoSpaceInfo,
        ),
    ];
    for (header_key, caption_key) in enabled_captioned {
        let headers = text_blocks_matching(&pump, strings.resolve(header_key));
        let captions = text_blocks_matching(&pump, strings.resolve(caption_key));
        assert_eq!(headers.len(), 1, "{header_key:?}");
        assert_eq!(captions.len(), 1, "{caption_key:?}");
        assert_eq!(headers[0].1, Some(1.0), "enabled header {header_key:?}");
        assert_eq!(
            captions[0].1,
            Some(cards::SECONDARY_OPACITY),
            "enabled caption {caption_key:?}"
        );
    }
}
