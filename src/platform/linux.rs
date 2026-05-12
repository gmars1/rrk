use std::io;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use evdev::Device;
use tokio::sync::broadcast;

use super::{CoreEvent, Error, Listener};

pub struct EvdevListener {
    tx: broadcast::Sender<CoreEvent>,
    handle: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    started: bool,
}

impl EvdevListener {
    pub fn new() -> io::Result<Self> {
        let (tx, _) = broadcast::channel(1024);
        Ok(Self {
            tx,
            handle: None,
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started: false,
        })
    }

    fn find_devices() -> Vec<Device> {
        let mut devices = Vec::new();
        for (_path, mut dev) in evdev::enumerate() {
            if dev.supported_keys().is_some() {
                let name = dev.name().unwrap_or("").to_lowercase();
                let skip = [
                    "mouse",
                    "touchpad",
                    "trackpoint",
                    "power",
                    "button",
                    "video",
                    "lid",
                    "tablet",
                    "joystick",
                ];
                if !skip.iter().any(|s| name.contains(s)) || name.is_empty() {
                    let _ = dev.grab();
                    devices.push(dev);
                }
            }
        }
        if devices.is_empty() {
            if let Ok(entries) = std::fs::read_dir("/dev/input") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.to_string_lossy().contains("event") {
                        if let Ok(mut dev) = Device::open(&path) {
                            if dev.supported_keys().is_some() {
                                let _ = dev.grab();
                                devices.push(dev);
                            }
                        }
                    }
                }
            }
        }
        devices
    }
}

impl Listener for EvdevListener {
    fn start(&mut self) -> Result<(), Error> {
        if self.started {
            return Ok(());
        }
        let tx = self.tx.clone();
        let stop = self.stop_flag.clone();

        let handle = thread::spawn(move || {
            // Use evdev::enumerate() to find keyboard devices
            let mut devices = Self::find_devices();

            if devices.is_empty() {
                eprintln!("[kbheat] No keyboard devices found — will use demo mode");
                // Device list is empty; loop will just sleep until stopped
            } else {
                eprintln!("[kbheat] Found {} keyboard device(s)", devices.len());
            }

            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                for dev in &mut devices {
                    if let Ok(events) = dev.fetch_events() {
                        for ev in events {
                            // value=1 means key-down
                            if ev.value() == 1 {
                                let _ = tx.send(CoreEvent::KeyPress(ev.code()));
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(5));
            }
        });

        self.handle = Some(handle);
        self.started = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Error> {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.started = false;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.started
    }

    fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.tx.subscribe()
    }
}
