//! The candidate window, shared by both shells. This process does not draw:
//! it spawns `taigikeyboard-popup` and writes one JSON command per line.
//! Clicks come back on the child's stdout and wake the shell through a
//! pipe — Fcitx5 watches the read end, IBus blocks a thread on it.
//!
//! The window places itself on X11. A Wayland session still gets the
//! window, and may not be able to put it on the caret.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command as Process, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

use taigi_desktop_core::composing::CandidateCellContent;
use taigi_desktop_core::session::LookupTableContent;
use taigi_desktop_core::settings::{keys, SettingsDocument};
use taigi_linux_platform::{paths::POPUP_BINARY_NAME, InstallLayout};
pub use taigikeyboard_popup::{Caret, Event};
use taigikeyboard_popup::{Cell, Chrome, Command, Show};

/// Primary and secondary sizes, typefaces and colors for one page. The
/// primary size is `hanji_px` (settings key `hanjiFontSize`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupStyle {
    pub hanji_px: u32,
    pub roman_px: u32,
    /// The leading script of a two-script cell is Hanji.
    pub hanji_leads: bool,
    pub chrome: Chrome,
}

impl PopupStyle {
    /// The shipped look, used when no settings document is open yet.
    pub fn standard() -> Self {
        Self {
            hanji_px: 16,
            roman_px: 16,
            hanji_leads: true,
            chrome: Chrome {
                follow_system: true,
                ..Chrome::default()
            },
        }
    }

    pub fn from_document(document: &SettingsDocument) -> Self {
        Self {
            hanji_px: clamp_px(
                document.i64(&keys::HANJI_FONT_SIZE),
                keys::CANDIDATE_FONT_PX_RANGE,
            ),
            roman_px: clamp_px(
                document.i64(&keys::ROMAN_FONT_SIZE),
                keys::CANDIDATE_FONT_PX_RANGE,
            ),
            hanji_leads: document.engine_settings().is_hanji_first,
            chrome: chrome_from(document),
        }
    }
}

fn chrome_from(document: &SettingsDocument) -> Chrome {
    let padding = clamp_px(
        document.i64(&keys::CANDIDATE_WINDOW_PADDING),
        keys::CANDIDATE_CHROME_PX_RANGE,
    );
    let radius = clamp_px(
        document.i64(&keys::CANDIDATE_CORNER_RADIUS),
        keys::CANDIDATE_CHROME_PX_RANGE,
    );
    let colors = keys::candidate_colors(document);
    // Default's four colors are what the window paints when the desktop
    // portal cannot be read. A named theme or custom colors are final.
    let follow_system = keys::shown_candidate_color_theme(document)
        == taigi_desktop_core::settings::CandidateColorTheme::Default;
    Chrome {
        padding,
        radius,
        hanji_family: document.string(&keys::HANJI_FONT_FAMILY),
        roman_family: document.string(&keys::ROMAN_FONT_FAMILY),
        text_color: colors.text,
        selected_text_color: colors.selected_text,
        background_color: colors.background,
        selected_background_color: colors.selected_background,
        follow_system,
        highlight_fill: clamp_px(
            document.i64(&keys::CANDIDATE_HIGHLIGHT_FILL),
            keys::CANDIDATE_HIGHLIGHT_FILL_RANGE,
        ),
    }
}

fn clamp_px(value: i64, range: std::ops::RangeInclusive<i64>) -> u32 {
    value.clamp(*range.start(), *range.end()) as u32
}

/// Shows `content`'s current page. An empty list hides the window.
pub fn present(content: &LookupTableContent, style: PopupStyle, caret: Caret) {
    let Some(client) = client() else {
        return;
    };
    let Some(command) = page_command(content, style, caret) else {
        hide();
        return;
    };
    if client.send(&command) {
        client.shown.store(true, Ordering::Relaxed);
    }
}

pub fn hide() {
    let Some(client) = client() else {
        return;
    };
    client.shown.store(false, Ordering::Relaxed);
    let _ = client.send(&Command::Hide);
}

/// Moves the window when it is up. A move before the first show is kept by
/// the shell and sent with that show.
pub fn move_to(caret: Caret) {
    let Some(client) = client() else {
        return;
    };
    if !client.shown.load(Ordering::Relaxed) {
        return;
    }
    let _ = client.send(&Command::Move { caret });
}

/// The read end Fcitx5 watches. Valid for the life of the process; the
/// caller does not close it. Made non-blocking, so [`poll_one`] never waits.
pub fn notify_fd() -> i32 {
    let Some(client) = client() else {
        return -1;
    };
    let reader = client
        .reader
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if set_nonblocking(&reader).is_err() {
        return -1;
    }
    use std::os::unix::io::AsRawFd;
    reader.as_raw_fd()
}

/// One queued event, or nothing. Reads the notify pipe when the queue is
/// empty so a level-triggered watcher does not spin.
pub fn poll_one() -> Option<Event> {
    let client = client()?;
    if let Some(event) = pop(client) {
        return Some(event);
    }
    read_available(client);
    pop(client)
}

/// Blocks until the window reports something. An empty vec is a leftover
/// wake; the caller waits again.
pub fn wait() -> Vec<Event> {
    let Some(client) = client() else {
        return Vec::new();
    };
    {
        let mut reader = client
            .reader
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut byte = [0_u8; 1];
        if reader.read(&mut byte).is_err() {
            return Vec::new();
        }
    }
    let mut events = client
        .events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    events.drain(..).collect()
}

/// The command for the page `content.cursor` is on. `None` when there is
/// nothing to show. Labels are already the current page's.
pub fn page_command(
    content: &LookupTableContent,
    style: PopupStyle,
    caret: Caret,
) -> Option<Command> {
    if content.cells.is_empty() {
        return None;
    }
    let page_size = content.page_size.max(1) as usize;
    let last = content.cells.len() - 1;
    let cursor = (content.cursor as usize).min(last);
    let start = (cursor / page_size) * page_size;
    let end = (start + page_size).min(content.cells.len());
    let cells = content.cells[start..end]
        .iter()
        .map(cell_of)
        .collect::<Vec<_>>();
    let count = cells.len();
    Some(Command::Show {
        body: Show {
            cells,
            labels: content.labels.iter().take(count).cloned().collect(),
            cursor: content
                .cursor_visible
                .then_some(content.cursor as usize)
                .filter(|index| *index >= start && *index < end)
                .map(|index| (index - start) as u32),
            vertical: content.vertical,
            has_prev: start > 0,
            has_next: end < content.cells.len(),
            hanji_px: style.hanji_px,
            roman_px: style.roman_px,
            hanji_leads: style.hanji_leads,
            chrome: style.chrome,
            caret,
        },
    })
}

fn cell_of(cell: &CandidateCellContent) -> Cell {
    Cell {
        text: cell.text.clone(),
        annotation: cell.annotation.clone(),
    }
}

struct Host {
    events: Mutex<VecDeque<Event>>,
    reader: Mutex<os_pipe::PipeReader>,
    writer: Mutex<os_pipe::PipeWriter>,
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Option<Child>>,
    shown: AtomicBool,
}

fn client() -> Option<&'static Host> {
    CLIENT.get_or_init(Host::open).as_ref()
}

fn set_nonblocking(reader: &os_pipe::PipeReader) -> std::io::Result<()> {
    let flags = rustix::fs::fcntl_getfl(reader)?;
    rustix::fs::fcntl_setfl(reader, flags | rustix::fs::OFlags::NONBLOCK)?;
    Ok(())
}

fn pop(client: &Host) -> Option<Event> {
    client
        .events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .pop_front()
}

fn read_available(client: &Host) {
    let mut reader = client
        .reader
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut buffer = [0_u8; 64];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

static CLIENT: OnceLock<Option<Host>> = OnceLock::new();

impl Host {
    fn open() -> Option<Self> {
        let (reader, writer) = match os_pipe::pipe() {
            Ok(ends) => ends,
            Err(error) => {
                log::error!("popup.pipe_failed error={error}");
                return None;
            }
        };
        Some(Self {
            events: Mutex::new(VecDeque::new()),
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            stdin: Mutex::new(None),
            child: Mutex::new(None),
            shown: AtomicBool::new(false),
        })
    }

    fn send(&self, command: &Command) -> bool {
        if !self.ensure() {
            return false;
        }
        let Ok(mut line) = serde_json::to_string(command) else {
            return false;
        };
        line.push('\n');
        let mut stdin = self
            .stdin
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(pipe) = stdin.as_mut() else {
            return false;
        };
        if pipe
            .write_all(line.as_bytes())
            .and_then(|()| pipe.flush())
            .is_ok()
        {
            return true;
        }
        drop(stdin);
        self.stop();
        if !self.ensure() {
            return false;
        }
        let mut stdin = self
            .stdin
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(pipe) = stdin.as_mut() else {
            return false;
        };
        pipe.write_all(line.as_bytes())
            .and_then(|()| pipe.flush())
            .is_ok()
    }

    fn ensure(&self) -> bool {
        {
            let mut child = self
                .child
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(running) = child.as_mut() {
                match running.try_wait() {
                    Ok(None) => return true,
                    _ => {
                        *child = None;
                    }
                }
            }
        }
        self.spawn()
    }

    fn stop(&self) {
        let mut stdin = self
            .stdin
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *stdin = None;
        let mut child = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(mut running) = child.take() {
            let _ = running.kill();
            let _ = running.wait();
        }
    }

    fn spawn(&self) -> bool {
        let Some(binary) = locate() else {
            log::error!("popup.missing");
            return false;
        };
        let mut process = match Process::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(process) => process,
            Err(error) => {
                log::error!("popup.spawn_failed path={} error={error}", binary.display());
                return false;
            }
        };
        let Some(stdout) = process.stdout.take() else {
            return false;
        };
        let Some(stdin) = process.stdin.take() else {
            return false;
        };
        let Ok(writer) = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .try_clone()
        else {
            return false;
        };
        let events = EventsHandle {
            host: client_pointer(),
        };
        thread::Builder::new()
            .name("taigi-popup-stdout".into())
            .spawn(move || read_stdout(stdout, writer, events))
            .ok();
        *self
            .stdin
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(stdin);
        *self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(process);
        true
    }
}

/// The reader thread cannot name the `OnceLock` while `open` is still
/// running, and it starts only after `open` has published the host. A
/// raw pointer to the leaked host would need `unsafe`. The thread instead
/// receives clones of the two pieces it writes.
struct EventsHandle {
    host: Option<&'static Host>,
}

fn client_pointer() -> Option<&'static Host> {
    CLIENT.get().and_then(Option::as_ref)
}

fn read_stdout(
    mut stdout: std::process::ChildStdout,
    mut writer: os_pipe::PipeWriter,
    events: EventsHandle,
) {
    let mut lines = std::io::BufReader::new(&mut stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match std::io::BufRead::read_line(&mut lines, &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Ok(event) = serde_json::from_str::<Event>(line.trim()) else {
            continue;
        };
        let Some(host) = events.host else {
            continue;
        };
        host.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(event);
        let _ = writer.write_all(&[1]);
        let _ = writer.flush();
    }
}

/// `TAIGIKEYBOARD_POPUP` when that file exists, else the binary beside this
/// process (an uninstalled IBus engine), else `<prefix>/bin/taigikeyboard-popup`.
pub fn locate() -> Option<PathBuf> {
    let override_path = std::env::var_os("TAIGIKEYBOARD_POPUP").map(PathBuf::from);
    let executable = std::env::current_exe().ok();
    resolve_binary(
        override_path.as_deref(),
        executable.as_deref(),
        &InstallLayout::shipped().popup_binary(),
    )
}

pub fn resolve_binary(
    override_path: Option<&Path>,
    executable: Option<&Path>,
    installed: &Path,
) -> Option<PathBuf> {
    if let Some(path) = override_path {
        if path.is_file() {
            return Some(path.to_owned());
        }
    }
    if let Some(executable) = executable {
        if let Some(directory) = executable.parent() {
            let sibling = directory.join(POPUP_BINARY_NAME);
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    installed.is_file().then(|| installed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::settings::CandidateColorTheme;

    fn cell(text: &str, annotation: Option<&str>) -> CandidateCellContent {
        CandidateCellContent::new(text, annotation.map(str::to_owned))
    }

    fn content(cursor: u32, vertical: bool) -> LookupTableContent {
        LookupTableContent {
            cells: (0..12)
                .map(|index| cell(&index.to_string(), None))
                .collect(),
            labels: vec!["q".into(), "w".into(), "e".into()],
            cursor,
            cursor_visible: true,
            page_size: 9,
            vertical,
        }
    }

    fn style() -> PopupStyle {
        PopupStyle::standard()
    }

    fn caret() -> Caret {
        Caret {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        }
    }

    #[test]
    fn the_page_around_the_cursor_is_what_is_shown() {
        let Command::Show { body } =
            page_command(&content(10, false), style(), caret()).expect("page")
        else {
            panic!("show");
        };
        assert_eq!(body.cells.len(), 3);
        assert_eq!(body.cells[0].text, "9");
        assert_eq!(body.cursor, Some(1));
        assert!(body.has_prev);
        assert!(!body.has_next);
        assert_eq!(body.labels, ["q", "w", "e"]);
        assert!(!body.vertical);
        assert_eq!(body.caret, caret());
    }

    #[test]
    fn an_empty_list_is_not_a_page() {
        let mut empty = content(0, true);
        empty.cells.clear();
        assert!(page_command(&empty, style(), caret()).is_none());
    }

    #[test]
    fn a_stored_size_outside_the_range_is_clamped() {
        let mut document = SettingsDocument::default();
        document.set_i64(&keys::HANJI_FONT_SIZE, 99);
        document.set_i64(&keys::ROMAN_FONT_SIZE, 1);
        let style = PopupStyle::from_document(&document);
        assert_eq!(style.hanji_px, 48);
        assert_eq!(style.roman_px, 12);
        document.set_i64(&keys::CANDIDATE_WINDOW_PADDING, -4);
        document.set_i64(&keys::CANDIDATE_CORNER_RADIUS, 99);
        document.set_string(&keys::CANDIDATE_TEXT_COLOR, "nope");
        document.set_choice(&keys::CANDIDATE_COLOR_THEME, CandidateColorTheme::Custom);
        document.set_string(&keys::HANJI_FONT_FAMILY, "Noto Serif CJK TC");
        let style = PopupStyle::from_document(&document);
        assert_eq!(style.chrome.padding, 0);
        assert_eq!(style.chrome.radius, 32);
        assert_eq!(style.chrome.text_color, 0x1A_1A_1A);
        assert_eq!(style.chrome.hanji_family, "Noto Serif CJK TC");
        document.set_i64(&keys::CANDIDATE_HIGHLIGHT_FILL, 150);
        let style = PopupStyle::from_document(&document);
        assert_eq!(style.chrome.highlight_fill, 100);
        document.set_i64(&keys::CANDIDATE_HIGHLIGHT_FILL, -3);
        let style = PopupStyle::from_document(&document);
        assert_eq!(style.chrome.highlight_fill, 0);
        assert!(!style.chrome.follow_system);
        document.set_choice(&keys::CANDIDATE_COLOR_THEME, CandidateColorTheme::Sakura);
        let style = PopupStyle::from_document(&document);
        assert_eq!(style.chrome.text_color, 0x1A_1A_1A);
        assert_eq!(style.chrome.selected_text_color, 0xFF_FFFF);
        assert_eq!(style.chrome.background_color, 0xE6_E5E3);
        assert_eq!(style.chrome.selected_background_color, 0xA5_3945);
        assert!(!style.chrome.follow_system);
    }

    #[test]
    fn the_default_theme_follows_the_desktop_and_keeps_the_shipped_fallback() {
        let style = PopupStyle::from_document(&SettingsDocument::default());
        assert!(style.chrome.follow_system);
        assert_eq!(style.chrome.background_color, 0xF6_F5F4);
        assert_eq!(style.chrome.selected_background_color, 0x1A_5FB4);
        assert!(PopupStyle::standard().chrome.follow_system);
    }

    #[test]
    fn the_override_wins_over_a_sibling_and_the_install() {
        let directory = tempfile::tempdir().expect("tempdir");
        let override_path = directory.path().join("override");
        let sibling_dir = directory.path().join("bin");
        std::fs::create_dir(&sibling_dir).expect("dir");
        let sibling = sibling_dir.join(POPUP_BINARY_NAME);
        let installed = directory.path().join("installed");
        std::fs::write(&override_path, b"x").expect("file");
        std::fs::write(&sibling, b"x").expect("file");
        std::fs::write(&installed, b"x").expect("file");
        let executable = sibling_dir.join("ibus-engine-taigikeyboard");
        assert_eq!(
            resolve_binary(Some(&override_path), Some(&executable), &installed),
            Some(override_path)
        );
        let missing = directory.path().join("missing");
        assert_eq!(
            resolve_binary(Some(&missing), Some(&executable), &installed),
            Some(sibling)
        );
        assert_eq!(resolve_binary(None, None, &installed), Some(installed));
    }
}
