use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tokio::sync::broadcast;

use super::{CoreEvent, Error, Listener};

pub struct MacListener {
    tx: broadcast::Sender<CoreEvent>,
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl MacListener {
    pub fn new() -> Result<Self, Error> {
        let (tx, _) = broadcast::channel(1024);
        Ok(Self {
            tx,
            handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    fn run(stop_flag: Arc<AtomicBool>, tx: broadcast::Sender<CoreEvent>) {
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

    fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.tx.subscribe()
    }
}
