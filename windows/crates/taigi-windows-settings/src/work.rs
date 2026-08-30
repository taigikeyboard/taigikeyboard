//! One piece of background work, and the answer when it lands. The update
//! check is the last caller: everything the window itself runs goes
//! through Reactor's `spawn_background`, which delivers a message rather
//! than being polled — but `UpdateState` is not a component and has no
//! context of its own, so it keeps a thread and a poll the window's beat
//! collects (roadmap W9).

// 中文: 更新檢查用的背景工作 — 一條執行緒 + 每次 tick 收一次結果(UpdateState 不是 component,拿不到 Reactor 的 spawn_background)。

use std::sync::mpsc;

/// A job in flight, and where its answer arrives.
pub struct PendingWork<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T: Send + 'static> PendingWork<T> {
    /// Runs `job` on its own thread; the caller polls for the answer.
    pub fn spawn_quiet(job: impl FnOnce() -> T + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            // A dropped receiver (the window went away) is not an error.
            let _ = sender.send(job());
        });
        Self { receiver }
    }

    /// The answer, once. `None` while the job runs; `Some(None)` when the
    /// job's thread died without answering (a panic) — a FAILURE the
    /// caller must show, never a quiet end.
    pub fn poll(&self) -> Option<Option<T>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(Some(value)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(None),
        }
    }
}
