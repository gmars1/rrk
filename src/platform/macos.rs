// macOS platform listener (stub implementation)
// Mirrors the Windows stub: provides a minimal Listener that periodically
// emits a dummy KeyPress event so the rest of the application can compile
// and run on macOS. Real HID handling can be added later.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{CoreEvent, Error, Listener};

/// Stub listener for macOS platforms.
pub struct MacListener {
    tx: Sender<CoreEvent>,
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl MacListener {
    pub fn new(tx: Sender<CoreEvent>) -> Self {
        Self {
            tx,
            handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    fn run(stop_flag: Arc<AtomicBool>, tx: Sender<CoreEvent>) {
        while !stop_flag.load(Ordering::SeqCst) {
            let _ = tx.send(CoreEvent::KeyPress(0));
            thread::sleep(Duration::from_secs(1));
        }
    }
}

impl Listener for MacListener {
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
        let (s, r) = mpsc::channel();
        let forward_tx = self.tx.clone();
        thread::spawn(move || {
            while let Ok(event) = forward_tx.recv() {
                let _ = s.send(event);
            }
        });
        r
    }
}
