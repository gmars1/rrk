// Windows platform listener (stub implementation)
// This listener follows the same `Listener` trait defined in `src/platform/mod.rs`.
// For now it provides a minimal functional implementation that spawns a thread,
// creates a channel for `CoreEvent`s and immediately returns without capturing any
// real key events. The structure mirrors the Linux implementation so that the rest
// of the application can compile and run on Windows. Real raw‑input handling can be
// added later without changing the public API.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{CoreEvent, Error, Listener};

/// Stub listener for Windows platforms.
///
/// It implements the `Listener` trait but does not actually listen to hardware
/// events. Instead it periodically sends a dummy `KeyPress` event (with a fake
/// key id) so that the rest of the system can be exercised during development
/// on Windows.
pub struct WinListener {
    tx: Sender<CoreEvent>,
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl WinListener {
    /// Create a new `WinListener`.
    ///
    /// The caller provides a `Sender<CoreEvent>` that will receive events from
    /// the background thread.
    pub fn new(tx: Sender<CoreEvent>) -> Self {
        Self {
            tx,
            handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Internal thread loop – currently just sleeps and sends a dummy event.
    fn run(stop_flag: Arc<AtomicBool>, tx: Sender<CoreEvent>) {
        // In a real implementation we would register for raw input and translate
        // Windows virtual‑key codes to our internal `KeyId`. Here we emit a fake
        // event every second so the UI has something to display.
        while !stop_flag.load(Ordering::SeqCst) {
            // Simulate a key press with an arbitrary id (e.g., 0).
            let _ = tx.send(CoreEvent::KeyPress(0));
            thread::sleep(Duration::from_secs(1));
        }
    }
}

impl Listener for WinListener {
    fn start(&mut self) -> Result<(), Error> {
        if self.handle.is_some() {
            return Ok(());
        }
        let stop = self.stop_flag.clone();
        let tx = self.tx.clone();
        self.handle = Some(thread::spawn(move || Self::run(stop, tx)));
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Error> {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.handle.is_some() && !self.stop_flag.load(Ordering::SeqCst)
    }

    fn subscribe(&mut self) -> Receiver<CoreEvent> {
        // The original design expects the listener to own the sending side and
        // give the caller a receiving end. Since we already have `tx` as a
        // `Sender`, we create a new channel and forward events.
        let (s, r) = mpsc::channel();
        let forward_tx = self.tx.clone();
        // Forward any events received on the internal `tx` to the new receiver.
        // This is a simple one‑way bridge; in a real implementation the listener
        // would write directly to the provided sender.
        thread::spawn(move || {
            while let Ok(event) = forward_tx.recv() {
                let _ = s.send(event);
            }
        });
        r
    }
}
