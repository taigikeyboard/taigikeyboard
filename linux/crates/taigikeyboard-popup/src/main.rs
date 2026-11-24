//! The candidate window. One process, spawned by either shell, spoken to
//! over stdin (a command per line) and stdout (an event per line).
//!
//! Placement is an override-redirect X11 window under the caret. A Wayland
//! session has no such placement: the window still draws, and may not sit
//! on the caret. GTK 4 cannot move a window, which is why this is not the
//! settings toolkit.

use std::io::{BufRead, Write};
use std::num::NonZeroU32;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;

use taigikeyboard_popup::paint;
use taigikeyboard_popup::text::{self, Fonts};
use taigikeyboard_popup::{
    arrange, paint_chrome, px, watch_appearance, Arrangement, Caret, Command, Event, Fit, Hit,
    Notice, Show, SystemAppearance, Target, SCREEN_MARGIN_PX,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowLevel};

const GAP_PX: f32 = 4.0;

/// A monitor: its origin and size, physical px.
type Monitor = (PhysicalPosition<i32>, PhysicalSize<u32>);

fn main() -> ExitCode {
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11();
    }
    let event_loop = match builder.build() {
        Ok(loop_) => loop_,
        Err(error) => {
            eprintln!("taigikeyboard-popup: {error}");
            return ExitCode::FAILURE;
        }
    };
    let proxy = event_loop.create_proxy();
    let appearance_proxy = proxy.clone();
    watch_appearance(move |appearance| {
        let _ = appearance_proxy.send_event(UserEvent::Appearance(appearance));
    });
    thread::Builder::new()
        .name("taigi-popup-stdin".into())
        .spawn(move || {
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Command>(&line) {
                    Ok(command) => {
                        if proxy.send_event(UserEvent::Command(command)).is_err() {
                            return;
                        }
                    }
                    Err(error) => eprintln!("taigikeyboard-popup: {error}"),
                }
            }
            let _ = proxy.send_event(UserEvent::Quit);
        })
        .ok();
    let mut app = App::default();
    if event_loop.run_app(&mut app).is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

enum UserEvent {
    Command(Command),
    Appearance(SystemAppearance),
    Quit,
}

struct App {
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    fonts: Fonts,
    page: Option<Show>,
    caret: Caret,
    arrangement: Option<Arrangement>,
    cursor: PhysicalPosition<f64>,
    pending: Vec<Command>,
    /// Last origin and size actually sent to the window. Arrow keys repaint
    /// the same caret; configuring again walks an override-redirect window.
    placed: Option<PhysicalPosition<i32>>,
    placed_size: Option<PhysicalSize<u32>>,
    appearance: Option<SystemAppearance>,
    /// The first row a vertical list taller than its room shows; kept while
    /// the page stays the same so the rows scroll only with the cursor.
    first_row: usize,
    /// The screen width last reported to the parent ([`Notice::Screen`]).
    reported_screen: Option<u32>,
    /// The room and scale the shown arrangement was laid out for: a caret
    /// move that keeps both only moves the window.
    laid_for: Option<(Fit, f32)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            fonts: Fonts::load(),
            page: None,
            caret: Caret {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            },
            arrangement: None,
            cursor: PhysicalPosition::new(0.0, 0.0),
            pending: Vec::new(),
            placed: None,
            placed_size: None,
            appearance: None,
            first_row: 0,
            reported_screen: None,
            laid_for: None,
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("TaigiKeyboard")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop);
        #[cfg(target_os = "linux")]
        let attributes = {
            use winit::platform::x11::WindowAttributesExtX11;
            attributes
                .with_override_redirect(true)
                .with_name("taigikeyboard-popup", "taigikeyboard")
        };
        let Ok(window) = event_loop.create_window(attributes) else {
            eprintln!("taigikeyboard-popup: the window could not be created");
            event_loop.exit();
            return;
        };
        let window = Arc::new(window);
        let Ok(context) = softbuffer::Context::new(window.clone()) else {
            eprintln!("taigikeyboard-popup: the window surface could not be created");
            event_loop.exit();
            return;
        };
        let Ok(surface) = softbuffer::Surface::new(&context, window.clone()) else {
            eprintln!("taigikeyboard-popup: the window surface could not be created");
            event_loop.exit();
            return;
        };
        self.window = Some(window);
        self.surface = Some(surface);
        // The session packs its first rows before any caret arrives.
        let primary = event_loop
            .primary_monitor()
            .map(|monitor| (monitor.position(), monitor.size()));
        self.report_screen(primary);
        let pending = std::mem::take(&mut self.pending);
        for command in pending {
            self.apply(event_loop, command);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.hide();
            }
            WindowEvent::RedrawRequested => self.paint(),
            WindowEvent::CursorMoved { position, .. } => self.cursor = position,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click_at(self.cursor),
            WindowEvent::ScaleFactorChanged { .. } => self.relayout(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Quit => event_loop.exit(),
            UserEvent::Appearance(appearance) => {
                self.appearance = Some(appearance);
                if self
                    .page
                    .as_ref()
                    .is_some_and(|page| page.chrome.follow_system)
                {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            UserEvent::Command(command) => {
                if self.window.is_none() {
                    self.pending.push(command);
                    return;
                }
                self.apply(event_loop, command);
            }
        }
    }
}

impl App {
    fn apply(&mut self, event_loop: &ActiveEventLoop, command: Command) {
        match command {
            Command::Hide => self.hide(),
            // The room depends on the caret: a move that changes it lays the
            // page out again.
            Command::Move { caret } => {
                self.caret = caret;
                self.relayout(event_loop);
            }
            Command::Show { body } => {
                self.caret = body.caret;
                if self
                    .page
                    .as_ref()
                    .is_none_or(|page| page.cells != body.cells)
                {
                    self.first_row = 0;
                }
                self.page = Some(*body);
                self.laid_for = None;
                self.relayout(event_loop);
                if let Some(window) = &self.window {
                    window.set_visible(true);
                    window.request_redraw();
                }
            }
        }
    }

    fn hide(&mut self) {
        self.page = None;
        self.arrangement = None;
        self.placed = None;
        self.placed_size = None;
        self.laid_for = None;
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
    }

    fn relayout(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = monitor_for(event_loop, self.caret);
        self.report_screen(monitor);
        let Some(page) = self.page.clone() else {
            return;
        };
        let scale = self.scale();
        let fit = fit_for(self.caret, monitor, scale, self.first_row);
        if self.laid_for == Some((fit, scale)) {
            self.place(monitor);
            return;
        }
        self.fonts
            .set_families(&page.chrome.hanji_family, &page.chrome.roman_family);
        let arrangement = arrange(&page, &self.fonts, scale, fit);
        self.first_row = arrangement.first_row;
        let first_row = arrangement.first_row;
        self.laid_for = Some((Fit { first_row, ..fit }, scale));
        let size = PhysicalSize::new(px(arrangement.width), px(arrangement.height));
        if self.placed_size != Some(size) {
            if let Some(window) = &self.window {
                let _ = window.request_inner_size(size);
                self.placed_size = Some(size);
            }
        }
        self.arrangement = Some(arrangement);
        self.place(monitor);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// The window's scale factor; 1 before the window exists.
    fn scale(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .filter(|scale| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0)
    }

    fn place(&mut self, monitor: Option<Monitor>) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(arrangement) = &self.arrangement else {
            return;
        };
        let size = PhysicalSize::new(px(arrangement.width), px(arrangement.height));
        let origin = held_origin(
            self.placed,
            place_at(self.caret, size, monitor, self.scale()),
        );
        if self.placed == Some(origin) {
            return;
        }
        window.set_outer_position(origin);
        self.placed = Some(origin);
    }

    fn click_at(&self, at: PhysicalPosition<f64>) {
        let Some(arrangement) = &self.arrangement else {
            return;
        };
        let Some(hit) = hit_at(arrangement, at.x as f32, at.y as f32) else {
            return;
        };
        let event = match hit.target {
            Target::Cell(position) => Event::Click { position },
            Target::Page { forward } => Event::Page { forward },
        };
        write_line(serde_json::to_string(&event));
    }

    /// Tells the parent the width of `monitor` in px before scale, when it
    /// changed, so the session packs rows that fit it.
    fn report_screen(&mut self, monitor: Option<Monitor>) {
        let Some((_, size)) = monitor else {
            return;
        };
        let screen = (size.width as f32 / self.scale()).floor() as u32;
        if self.reported_screen == Some(screen) {
            return;
        }
        self.reported_screen = Some(screen);
        write_line(serde_json::to_string(&Notice::Screen { width: screen }));
    }

    fn paint(&mut self) {
        let (Some(surface), Some(arrangement)) = (self.surface.as_mut(), self.arrangement.as_ref())
        else {
            return;
        };
        let width = px(arrangement.width);
        let height = px(arrangement.height);
        let (Some(width), Some(height)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return;
        };
        if surface.resize(width, height).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let pixels = width.get() as usize * height.get() as usize;
        buffer[..pixels].fill(0);
        let chrome = self
            .page
            .as_ref()
            .map(|page| paint_chrome(page.chrome.clone(), self.appearance))
            .unwrap_or_default();
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        text::paint_arrangement(
            &self.fonts,
            &chrome,
            arrangement,
            scale,
            &mut buffer,
            (width.get(), height.get()),
        );
        // X11's ARGB visual is premultiplied. Straight alpha leaves a bright
        // fringe on the rounded corner.
        paint::premultiply(&mut buffer[..pixels]);
        let _ = buffer.present();
    }
}

fn write_line(line: serde_json::Result<String>) {
    if let Ok(line) = line {
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
    }
}

fn hit_at(arrangement: &Arrangement, x: f32, y: f32) -> Option<&Hit> {
    arrangement
        .hits
        .iter()
        .rev()
        .find(|hit| x >= hit.x && y >= hit.y && x < hit.x + hit.w && y < hit.y + hit.h)
}

fn monitor_for(event_loop: &ActiveEventLoop, caret: Caret) -> Option<Monitor> {
    let contains = |origin: PhysicalPosition<i32>, size: PhysicalSize<u32>| {
        caret.x >= origin.x
            && caret.y >= origin.y
            && caret.x < origin.x.saturating_add(size.width as i32)
            && caret.y < origin.y.saturating_add(size.height as i32)
    };
    let found = event_loop
        .available_monitors()
        .find(|monitor| contains(monitor.position(), monitor.size()));
    let monitor = found.or_else(|| event_loop.primary_monitor())?;
    Some(usable((monitor.position(), monitor.size()), work_area()))
}

/// `monitor` cut to the desktop's work area, as macOS keeps a panel inside
/// `visibleFrame`: the window never sits under a top bar or a dock. The
/// whole monitor when there is no work area or they do not overlap.
fn usable(monitor: Monitor, work_area: Option<Monitor>) -> Monitor {
    let Some((area_origin, area_size)) = work_area else {
        return monitor;
    };
    let (origin, size) = monitor;
    let left = origin.x.max(area_origin.x);
    let top = origin.y.max(area_origin.y);
    let right = (origin.x + size.width as i32).min(area_origin.x + area_size.width as i32);
    let bottom = (origin.y + size.height as i32).min(area_origin.y + area_size.height as i32);
    if right <= left || bottom <= top {
        return monitor;
    }
    (
        PhysicalPosition::new(left, top),
        PhysicalSize::new((right - left) as u32, (bottom - top) as u32),
    )
}

/// The X11 work area of the current desktop (`_NET_WORKAREA`, EWMH), in
/// root-window px. One connection for the life of the window; the property
/// is read again each time, since a dock that hides changes it.
#[cfg(target_os = "linux")]
fn work_area() -> Option<Monitor> {
    use std::sync::OnceLock;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    use x11rb::rust_connection::RustConnection;

    struct Root {
        connection: RustConnection,
        window: u32,
        work_area: u32,
        current_desktop: u32,
    }
    static ROOT: OnceLock<Option<Root>> = OnceLock::new();
    let root = ROOT
        .get_or_init(|| {
            let (connection, screen) = x11rb::connect(None).ok()?;
            let window = connection.setup().roots.get(screen)?.root;
            let atom = |name: &str| {
                connection
                    .intern_atom(false, name.as_bytes())
                    .ok()?
                    .reply()
                    .ok()
                    .map(|reply| reply.atom)
            };
            let work_area = atom("_NET_WORKAREA")?;
            let current_desktop = atom("_NET_CURRENT_DESKTOP")?;
            Some(Root {
                connection,
                window,
                work_area,
                current_desktop,
            })
        })
        .as_ref()?;
    let cardinals = |property: u32, offset: u32, length: u32| {
        let reply = root
            .connection
            .get_property(
                false,
                root.window,
                property,
                AtomEnum::CARDINAL,
                offset,
                length,
            )
            .ok()?
            .reply()
            .ok()?;
        let values: Vec<u32> = reply.value32()?.collect();
        Some(values)
    };
    // GNOME's XWayland publishes no current desktop: the first area.
    let desktop = cardinals(root.current_desktop, 0, 1)
        .and_then(|values| values.first().copied())
        .unwrap_or(0);
    let area = cardinals(root.work_area, desktop * 4, 4)?;
    let [x, y, width, height] = area[..] else {
        return None;
    };
    Some((
        PhysicalPosition::new(x as i32, y as i32),
        PhysicalSize::new(width, height),
    ))
}

#[cfg(not(target_os = "linux"))]
fn work_area() -> Option<Monitor> {
    None
}

/// Candidate navigation reports a caret a few pixels off the one the window
/// was opened on. Sending that to X11 again shifts an override-redirect
/// window. Hold the anchor until the caret actually moves.
fn held_origin(
    anchor: Option<PhysicalPosition<i32>>,
    next: PhysicalPosition<i32>,
) -> PhysicalPosition<i32> {
    const SLACK: i32 = 4;
    match anchor {
        Some(prev) if (next.x - prev.x).abs() <= SLACK && (next.y - prev.y).abs() <= SLACK => prev,
        _ => next,
    }
}

/// The gap from the caret and the margin from the monitor's edges, whole
/// physical px.
fn spacing(scale: f32) -> (i32, i32) {
    let whole = |value: f32| (value * scale).round().max(1.0) as i32;
    (whole(GAP_PX), whole(SCREEN_MARGIN_PX))
}

/// The room under and over `caret` on `monitor`, physical px.
fn vertical_room(caret: Caret, (origin, size): Monitor, scale: f32) -> (i32, i32) {
    let (gap, margin) = spacing(scale);
    let bottom = origin.y.saturating_add(size.height as i32) - margin;
    let below = bottom - caret.y.saturating_add(caret.h).saturating_add(gap);
    let above = caret.y.saturating_sub(gap) - (origin.y + margin);
    (below, above)
}

/// The room [`place_at`] can give the window: the monitor's width less its
/// margins, and the roomier side of the caret. No monitor: unbounded.
fn fit_for(caret: Caret, monitor: Option<Monitor>, scale: f32, first_row: usize) -> Fit {
    let Some(monitor) = monitor else {
        return Fit {
            first_row,
            ..Fit::UNBOUNDED
        };
    };
    let (_, margin) = spacing(scale);
    let (below, above) = vertical_room(caret, monitor, scale);
    Fit {
        width: (monitor.1.width as i32 - margin * 2).max(1) as f32,
        height: below.max(above).max(1) as f32,
        first_row,
    }
}

/// Under the caret when the window fits there, else over it, else on the
/// roomier side. A window [`fit_for`] sized never covers the caret's line;
/// an unfolded grid, which takes no fit, may. Kept inside the monitor's
/// margins.
fn place_at(
    caret: Caret,
    window: PhysicalSize<u32>,
    monitor: Option<Monitor>,
    scale: f32,
) -> PhysicalPosition<i32> {
    let (gap, margin) = spacing(scale);
    let below_y = caret.y.saturating_add(caret.h).saturating_add(gap);
    let mut x = caret.x;
    let Some((origin, size)) = monitor else {
        return PhysicalPosition::new(x, below_y);
    };
    let right = origin.x.saturating_add(size.width as i32);
    let window_w = window.width as i32;
    let window_h = window.height as i32;
    if x.saturating_add(window_w) > right - margin {
        x = (right - margin).saturating_sub(window_w);
    }
    if x < origin.x + margin {
        x = origin.x + margin;
    }
    let (below, above) = vertical_room(caret, (origin, size), scale);
    let above_y = caret.y.saturating_sub(gap).saturating_sub(window_h);
    let y = if window_h <= below {
        below_y
    } else if window_h <= above || above > below {
        above_y
    } else {
        below_y
    };
    // An unfolded grid takes no fit and can outgrow both sides; it still
    // stays on the monitor.
    let bottom = origin.y.saturating_add(size.height as i32) - margin;
    let y = y.min(bottom - window_h).max(origin.y + margin);
    PhysicalPosition::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_few_pixels_of_caret_jitter_do_not_move_the_window() {
        let anchor = PhysicalPosition::new(100, 200);
        assert_eq!(
            held_origin(Some(anchor), PhysicalPosition::new(103, 201)),
            anchor
        );
        assert_eq!(
            held_origin(Some(anchor), PhysicalPosition::new(110, 200)),
            PhysicalPosition::new(110, 200)
        );
        assert_eq!(
            held_origin(None, PhysicalPosition::new(110, 200)),
            PhysicalPosition::new(110, 200)
        );
    }

    const MONITOR: Monitor = (PhysicalPosition::new(0, 0), PhysicalSize::new(1280, 800));

    fn caret_at(y: i32) -> Caret {
        Caret {
            x: 100,
            y,
            w: 2,
            h: 30,
        }
    }

    fn covers_caret(origin: PhysicalPosition<i32>, height: u32, caret: Caret) -> bool {
        origin.y < caret.y + caret.h && origin.y + height as i32 > caret.y
    }

    /// L2: a window taller than either side of the caret fell back to the
    /// screen's bottom edge, over the caret's line. The fit now caps it at the
    /// roomier side, where it is placed.
    #[test]
    fn a_tall_window_takes_the_roomier_side_and_never_covers_the_caret() {
        // trace: caret 300..330, gap 4, margin 4: below = 796 - 334 = 462,
        // above = 296 - 4 = 292.
        let caret = caret_at(300);
        let fit = fit_for(caret, Some(MONITOR), 1.0, 0);
        assert_eq!(fit.height, 462.0);
        assert_eq!(fit.width, 1272.0);
        let origin = place_at(caret, PhysicalSize::new(300, 462), Some(MONITOR), 1.0);
        assert_eq!(origin.y, 334);
        assert!(!covers_caret(origin, 462, caret));

        // trace: caret 600..630: below = 796 - 634 = 162, above = 596 - 4 = 592.
        let low = caret_at(600);
        let fit = fit_for(low, Some(MONITOR), 1.0, 0);
        assert_eq!(fit.height, 592.0);
        let origin = place_at(low, PhysicalSize::new(300, 592), Some(MONITOR), 1.0);
        assert_eq!(origin.y, 4);
        assert!(!covers_caret(origin, 592, low));
    }

    #[test]
    fn a_window_that_fits_below_stays_below() {
        let caret = caret_at(300);
        let origin = place_at(caret, PhysicalSize::new(300, 200), Some(MONITOR), 1.0);
        assert_eq!(origin, PhysicalPosition::new(100, 334));
    }

    #[test]
    fn a_window_taller_than_the_monitor_stays_on_it() {
        // An unfolded grid takes no fit: 1000 px on an 800 px monitor.
        let origin = place_at(
            caret_at(300),
            PhysicalSize::new(300, 1000),
            Some(MONITOR),
            1.0,
        );
        assert_eq!(origin.y, 4);
    }

    #[test]
    fn the_work_area_cuts_the_monitor() {
        // trace: GNOME on the VM publishes 66, 32, 1214 × 768 (dock, top bar).
        let area = (PhysicalPosition::new(66, 32), PhysicalSize::new(1214, 768));
        assert_eq!(usable(MONITOR, Some(area)), area);
        assert_eq!(usable(MONITOR, None), MONITOR);
        // A work area on another monitor leaves this one whole.
        let elsewhere = (
            PhysicalPosition::new(1280, 0),
            PhysicalSize::new(1920, 1080),
        );
        assert_eq!(usable(MONITOR, Some(elsewhere)), MONITOR);
    }
}
