//! The Manage Dictionaries search the Swift settings pane asks (#792):
//! desktop-core's one lookup (`engine::dictionary_search::search`, the
//! Windows and Linux one) under the settings snapshot the request carries,
//! read for its one answer and never put in force. No session, no Shell
//! lock — `Shell::serve` answers it before taking one — so a search sent
//! from a background thread never waits on, or holds up, the key path; the
//! engine's own per-request locks order the two.

use taigi_desktop_core::engine::dictionary_search::{search, DictionarySearchResult};
use taigi_desktop_core::settings::dictionary_search::VISIBLE_RESULT_LIMIT;

use crate::key_rules;
use crate::proto::{
    DictionarySearchHit, DictionarySearchReply, DictionarySearchRequest, SettingsSnapshot,
};
use crate::runtime::Refusal;

pub(crate) fn answer(
    request: &DictionarySearchRequest,
    snapshot: Option<&SettingsSnapshot>,
) -> Result<DictionarySearchReply, Refusal> {
    let hits = search(request.query.trim(), &key_rules::document(snapshot)?)
        .into_iter()
        .take(VISIBLE_RESULT_LIMIT)
        .map(hit)
        .collect();
    Ok(DictionarySearchReply { hits })
}

fn hit(result: DictionarySearchResult) -> DictionarySearchHit {
    DictionarySearchHit {
        key: result.key(),
        badge_keys: result
            .badge_keys()
            .into_iter()
            .map(|key| key.as_str().to_owned())
            .collect(),
        moe_url: result.moe_url(),
        chhoe_url: result.chhoe_url(),
        roman: result.roman,
        hanji: result.hanji,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{boolean, engine_shell, text};
    use taigi_desktop_core::strings::StringKey;

    /// The MOE dictionary's badge, as the reply spells it.
    fn moe_badge() -> &'static str {
        StringKey::DictionaryKautianTag.as_str()
    }

    fn search_under(
        query: &str,
        entries: Vec<crate::proto::SettingEntry>,
    ) -> DictionarySearchReply {
        let (_engine, shell) = engine_shell();
        let request =
            crate::proto::desktop_request::Request::DictionarySearch(DictionarySearchRequest {
                query: query.to_owned(),
            });
        match shell.serve(request, Some(&SettingsSnapshot { entries })) {
            Ok(crate::proto::desktop_response::Reply::DictionarySearch(reply)) => reply,
            other => panic!("expected a search reply, got {other:?}"),
        }
    }

    /// trace: the shipped MOE dictionary holds 食 / tsia̍h, so "tsiah" finds
    /// it, with the MOE badge as its i18n key and both lookup pages; the
    /// reply never holds more rows than the pane shows.
    #[test]
    fn a_reading_finds_its_word_with_badges_and_lookups() {
        let reply = search_under("tsiah", vec![text("inputMode", "tl")]);
        assert!(!reply.hits.is_empty() && reply.hits.len() <= VISIBLE_RESULT_LIMIT);
        let hit = reply
            .hits
            .iter()
            .find(|hit| hit.hanji.as_deref() == Some("食"))
            .unwrap_or_else(|| panic!("食 among {:?}", reply.hits));
        assert!(hit.badge_keys.iter().any(|badge| badge == moe_badge()));
        assert!(hit.moe_url.is_some() && hit.chhoe_url.is_some());
        assert!(hit.key.starts_with("system:"));
    }

    /// The query is trimmed, and an empty one finds nothing.
    #[test]
    fn a_blank_query_finds_nothing() {
        assert!(search_under("   ", vec![]).hits.is_empty());
    }

    /// The toggles the snapshot carries are the ones the search reads: with the
    /// MOE dictionary off, its badge is gone.
    #[test]
    fn the_carried_toggles_filter_the_badges() {
        let reply = search_under(
            "tsiah",
            vec![text("inputMode", "tl"), boolean("moeDictEnabled", false)],
        );
        assert!(!reply.hits.is_empty(), "the other sources still find it");
        assert!(reply
            .hits
            .iter()
            .all(|hit| !hit.badge_keys.iter().any(|badge| badge == moe_badge())));
    }
}
