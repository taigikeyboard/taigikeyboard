//! `org.freedesktop.portal.Settings`, namespace `org.freedesktop.appearance`.
//! `color-scheme` is `u` (1 prefers dark; 0, 2 and anything else are light).
//! `accent-color` is `(ddd)` in sRGB, range `[0, 1]`; a component outside
//! that range means the accent is unset. A portal that cannot be reached
//! leaves the window on the colors the parent sent.

use std::thread;

use zbus::blocking::Connection;
use zbus::proxy;
use zbus::zvariant::Value;

use crate::system_color::SystemAppearance;

const NAMESPACE: &str = "org.freedesktop.appearance";
const COLOR_SCHEME: &str = "color-scheme";
const ACCENT_COLOR: &str = "accent-color";

#[proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    gen_async = false
)]
trait Settings {
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<zbus::zvariant::OwnedValue>;

    #[zbus(signal)]
    fn setting_changed(&self, namespace: &str, key: &str, value: Value<'_>) -> zbus::Result<()>;
}

/// Reads the appearance once, then again whenever either key changes.
/// Calls `on_change` on this thread. Returns when the bus is gone.
pub fn watch_appearance(on_change: impl Fn(SystemAppearance) + Send + 'static) {
    thread::Builder::new()
        .name("taigi-popup-portal".into())
        .spawn(move || {
            let Ok(connection) = Connection::session() else {
                eprintln!("taigikeyboard-popup: desktop settings are unavailable");
                return;
            };
            let Ok(proxy) = SettingsProxy::new(&connection) else {
                eprintln!("taigikeyboard-popup: desktop settings are unavailable");
                return;
            };
            if let Some(appearance) = read_appearance(&proxy) {
                on_change(appearance);
            }
            let Ok(changes) = proxy.receive_setting_changed() else {
                return;
            };
            for change in changes {
                let Ok(args) = change.args() else {
                    continue;
                };
                if *args.namespace() != NAMESPACE {
                    continue;
                }
                let key = args.key();
                if *key != COLOR_SCHEME && *key != ACCENT_COLOR {
                    continue;
                }
                if let Some(appearance) = read_appearance(&proxy) {
                    on_change(appearance);
                }
            }
        })
        .ok();
}

fn read_appearance(proxy: &SettingsProxy) -> Option<SystemAppearance> {
    let scheme = proxy.read(NAMESPACE, COLOR_SCHEME).ok()?;
    let accent = proxy.read(NAMESPACE, ACCENT_COLOR).ok();
    Some(appearance_from(&scheme, accent.as_deref()))
}

fn appearance_from(scheme: &Value<'_>, accent: Option<&Value<'_>>) -> SystemAppearance {
    SystemAppearance {
        prefers_dark: color_scheme(scheme) == Some(1),
        accent: accent.and_then(accent_rgb),
    }
}

fn color_scheme(value: &Value<'_>) -> Option<u32> {
    match unwrap(value) {
        Value::U32(scheme) => Some(*scheme),
        _ => None,
    }
}

/// `(ddd)` in `[0, 1]`. Anything else is an unset accent.
fn accent_rgb(value: &Value<'_>) -> Option<u32> {
    let Value::Structure(structure) = unwrap(value) else {
        return None;
    };
    let mut channels = [0.0_f64; 3];
    for (slot, field) in channels.iter_mut().zip(structure.fields()) {
        let Value::F64(channel) = unwrap(field) else {
            return None;
        };
        if !(0.0..=1.0).contains(channel) {
            return None;
        }
        *slot = *channel;
    }
    if structure.fields().len() != 3 {
        return None;
    }
    Some(
        pack_channel(channels[0]) << 16
            | pack_channel(channels[1]) << 8
            | pack_channel(channels[2]),
    )
}

fn unwrap<'a>(value: &'a Value<'a>) -> &'a Value<'a> {
    match value {
        Value::Value(inner) => unwrap(inner),
        other => other,
    }
}

fn pack_channel(channel: f64) -> u32 {
    (channel * 255.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheme(value: u32) -> Value<'static> {
        Value::U32(value)
    }

    fn accent(r: f64, g: f64, b: f64) -> Value<'static> {
        Value::Structure(zbus::zvariant::Structure::from((r, g, b)))
    }

    #[test]
    fn only_prefer_dark_is_dark_and_the_accent_is_srgb() {
        // trace: portal color-scheme 1 is prefer-dark; 0 and 2 are not.
        // (1, 0, 0.5) → (255, 0, 128) because round(0.5 * 255) = 128.
        let dark = appearance_from(&scheme(1), Some(&accent(1.0, 0.0, 0.5)));
        assert!(dark.prefers_dark);
        assert_eq!(dark.accent, Some(0xFF_0080));
        assert!(!appearance_from(&scheme(0), None).prefers_dark);
        assert!(!appearance_from(&scheme(2), None).prefers_dark);
        assert!(!appearance_from(&Value::U8(1), None).prefers_dark);
    }

    #[test]
    fn an_accent_outside_srgb_is_unset_and_a_wrapped_scheme_still_counts() {
        let wrapped = Value::Value(Box::new(scheme(1)));
        let over = accent(1.2, 0.0, 0.0);
        let appearance = appearance_from(&wrapped, Some(&over));
        assert!(appearance.prefers_dark);
        assert_eq!(appearance.accent, None);
        assert_eq!(
            accent_rgb(&Value::Structure(zbus::zvariant::Structure::from((
                0.0_f64,
            )))),
            None
        );
    }
}
