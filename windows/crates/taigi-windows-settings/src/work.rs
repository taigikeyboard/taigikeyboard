//! One piece of background work per page, and the chrome around it: a
//! store call runs on its own thread so a 30000-row import cannot freeze
//! the window showing its spinner, and the spinner itself appears only
//! after 400 ms so a millisecond-long write does not flash it
//! (`UserDataPageChrome.swift`). One slot per page: the page holds
//! `Option<PendingWork>`, and a second job while one runs is refused.

// 中文: 頁面的背景工作與其外觀 — 一頁一個工作槽,400ms 後才顯示轉圈。

use std::sync::mpsc;
use std::time::{Duration, Instant};
use taigi_windows_core::strings::{StringKey, StringResolver};

/// How long a job may run before the page says so (`overlayDelay`).
const OVERLAY_DELAY: Duration = Duration::from_millis(400);

/// A job in flight: what it is called, when it began, where its answer
/// arrives.
pub struct PendingWork<T> {
    /// What the overlay calls the job; `None` for a job the page never
    /// announces (a load).
    label: Option<StringKey>,
    started: Instant,
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> PendingWork<T> {
    /// Runs `job` on its own thread; the page polls for the answer.
    pub fn spawn(label: StringKey, job: impl FnOnce() -> T + Send + 'static) -> Self {
        Self::start(Some(label), job)
    }

    /// A job with no overlay: the page keeps drawing what it has until the
    /// answer replaces it.
    pub fn spawn_quiet(job: impl FnOnce() -> T + Send + 'static) -> Self {
        Self::start(None, job)
    }

    fn start(label: Option<StringKey>, job: impl FnOnce() -> T + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // A dropped receiver (the page went away) is not an error.
            let _ = sender.send(job());
        });
        Self {
            label,
            started: Instant::now(),
            receiver,
        }
    }

    /// The answer, once. `None` while the job runs; `Some(None)` when the
    /// job's thread died without answering (a panic) — a FAILURE the page
    /// must show, never a quiet end.
    pub fn poll(&self) -> Option<Option<T>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(Some(value)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(None),
        }
    }

    pub fn is_slow(&self) -> bool {
        self.started.elapsed() >= OVERLAY_DELAY
    }

    pub fn label(&self) -> Option<StringKey> {
        self.label
    }
}

/// The progress overlay: a spinner and the job's name, centred over the
/// page, once the job has run long enough to be worth saying so.
pub fn show_overlay<T: Send + 'static>(
    ctx: &egui::Context,
    strings: &StringResolver,
    work: Option<&PendingWork<T>>,
) {
    let Some(work) = work else {
        return;
    };
    // Keep the frames coming while the job runs, so the answer lands
    // promptly and the spinner turns.
    ctx.request_repaint_after(Duration::from_millis(50));
    let Some(label) = work.label() else {
        return;
    };
    if !work.is_slow() {
        return;
    }
    egui::Area::new(egui::Id::new("work_overlay"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .inner_margin(24.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.add_space(8.0);
                        ui.label(strings.resolve(label));
                    });
                });
        });
}
