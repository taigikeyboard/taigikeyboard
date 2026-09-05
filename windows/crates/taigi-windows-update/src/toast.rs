//! The "an update is available" notice as a Windows toast (`UpdateAnnouncement`
//! on the Mac, a `UNUserNotification`). Unpackaged-desktop toasts need an
//! Application User Model ID with a Start-menu shortcut carrying it — the
//! installer creates the shortcut (PR10); this only speaks the id. No
//! activation handler: the toast is informational, and the 一般 pane is the
//! surface that keeps working after it is dismissed (roadmap W9).

/// The Application User Model ID the installer stamps on the Start-menu
/// shortcut; a toast posted under any other id is silently dropped.
pub const APPLICATION_USER_MODEL_ID: &str = "TaigiKeyboard.Settings";

/// Shows a two-line toast; answers whether Windows ACCEPTED it — a missing
/// AUMID shortcut (registration fails) or a non-Windows host answer
/// `false`. Accepted is not seen: Focus Assist or a per-app notification
/// setting can still hold it back, which `Show` does not report (PR11
/// smoke item).
#[cfg(windows)]
pub fn show(title: &str, body: &str) -> bool {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape(title),
        escape(body)
    );
    let posted = (|| -> windows::core::Result<()> {
        let document = XmlDocument::new()?;
        document.LoadXml(&HSTRING::from(xml))?;
        let toast = ToastNotification::CreateToastNotification(&document)?;
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
            APPLICATION_USER_MODEL_ID,
        ))?;
        notifier.Show(&toast)
    })();
    match posted {
        Ok(()) => true,
        Err(error) => {
            log::warn!("update.toast_failed error={error}");
            false
        }
    }
}

#[cfg(not(windows))]
pub fn show(title: &str, body: &str) -> bool {
    log::info!("update.toast_stub title={title} body={body}");
    false
}

/// The five XML escapes; the texts are i18n strings and a version number.
#[cfg_attr(not(windows), allow(dead_code))]
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toast_text_is_xml_escaped() {
        assert_eq!(escape("a<b>&\"c'"), "a&lt;b&gt;&amp;&quot;c&apos;");
    }
}
