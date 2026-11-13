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
    arrange, paint_chrome, px, watch_appearance, Arrangement, Caret, Command, Event, Hit, Show,
    SystemAppearance, Target,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowLevel};

const GAP_PX: f32 = 4.0;

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
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attributes = Window::default_attributes()
            .with_title("TaigiKeyboard")
            .with_decorations(false)
            .with_resizable(false)
            .with_visible(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop);
        #[cfg(target_os = "linux")]
        {
            use winit::platform::x11::WindowAttributesExtX11;
            attributes = attributes
                .with_override_redirect(true)
                .with_name("taigikeyboard-popup", "taigikeyboard");
        }
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
            Command::Move { caret } => {
                self.caret = caret;
                self.place(event_loop);
            }
            Command::Show { body } => {
                self.caret = body.caret;
                self.page = Some(*body);
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
        if let Some(window) = &self.window {
            window.set_visible(false);
        }
    }

    fn relayout(&mut self, event_loop: &ActiveEventLoop) {
        let Some(page) = self.page.clone() else {
            return;
        };
        let scale = self
            .window
            .as_ref()
            .map(|window| window.scale_factor() as f32)
            .unwrap_or(1.0);
        self.fonts
            .set_families(&page.chrome.hanji_family, &page.chrome.roman_family);
        let arrangement = arrange(&page, &self.fonts, scale);
        let size = PhysicalSize::new(px(arrangement.width), px(arrangement.height));
        if self.placed_size != Some(size) {
            if let Some(window) = &self.window {
                let _ = window.request_inner_size(size);
                self.placed_size = Some(size);
            }
        }
        self.arrangement = Some(arrangement);
        self.place(event_loop);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn place(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(arrangement) = &self.arrangement else {
            return;
        };
        let size = PhysicalSize::new(px(arrangement.width), px(arrangement.height));
        let scale = window.scale_factor() as f32;
        let gap = (GAP_PX * scale).round().max(1.0) as i32;
        let monitor = monitor_for(event_loop, self.caret);
        let origin = held_origin(self.placed, place_at(self.caret, size, monitor, gap));
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
        if let Ok(line) = serde_json::to_string(&event) {
            let mut stdout = std::io::stdout().lock();
            let _ = writeln!(stdout, "{line}");
            let _ = stdout.flush();
        }
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

fn hit_at(arrangement: &Arrangement, x: f32, y: f32) -> Option<&Hit> {
    arrangement
        .hits
        .iter()
        .rev()
        .find(|hit| x >= hit.x && y >= hit.y && x < hit.x + hit.w && y < hit.y + hit.h)
}

fn monitor_for(
    event_loop: &ActiveEventLoop,
    caret: Caret,
) -> Option<(PhysicalPosition<i32>, PhysicalSize<u32>)> {
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
    Some((monitor.position(), monitor.size()))
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

fn place_at(
    caret: Caret,
    window: PhysicalSize<u32>,
    monitor: Option<(PhysicalPosition<i32>, PhysicalSize<u32>)>,
    gap: i32,
) -> PhysicalPosition<i32> {
    let mut x = caret.x;
    let mut y = caret.y.saturating_add(caret.h).saturating_add(gap);
    let Some((origin, size)) = monitor else {
        return PhysicalPosition::new(x, y);
    };
    let right = origin.x.saturating_add(size.width as i32);
    let bottom = origin.y.saturating_add(size.height as i32);
    let window_w = window.width as i32;
    let window_h = window.height as i32;
    if x.saturating_add(window_w) > right {
        x = right.saturating_sub(window_w);
    }
    if x < origin.x {
        x = origin.x;
    }
    if y.saturating_add(window_h) > bottom {
        let above = caret.y.saturating_sub(gap).saturating_sub(window_h);
        y = if above >= origin.y {
            above
        } else {
            bottom.saturating_sub(window_h)
        };
    }
    if y < origin.y {
        y = origin.y;
    }
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
}
