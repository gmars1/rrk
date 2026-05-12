use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tokio::sync::broadcast;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::{CoreEvent, Error, Listener};

static EVENT_TX: Mutex<Option<broadcast::Sender<CoreEvent>>> = Mutex::new(None);

pub struct WinListener {
    tx: broadcast::Sender<CoreEvent>,
    handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl WinListener {
    pub fn new() -> Result<Self, Error> {
        let (tx, _) = broadcast::channel(1024);
        Ok(Self {
            tx,
            handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    fn map_scan_code(make_code: u16, is_extended: bool) -> Option<u16> {
        if is_extended {
            match make_code {
                0x1D => Some(97),  // Right Ctrl   → KEY_RIGHTCTRL
                0x38 => Some(100), // Right Alt    → KEY_RIGHTALT
                0x5B => Some(125), // Left Win     → KEY_LEFTMETA
                0x5C => Some(126), // Right Win    → KEY_RIGHTMETA
                0x5D => Some(127), // Menu/Apps    → KEY_COMPOSE
                _ => None,
            }
        } else {
            Some(make_code)
        }
    }

    unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let msg = wparam.0 as u32;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let is_extended = (kb.flags.0 & LLKHF_EXTENDED.0) != 0;
                if let Some(evdev) = Self::map_scan_code(kb.scanCode as u16, is_extended) {
                    if let Some(tx) = EVENT_TX.lock().unwrap().as_ref() {
                        let _ = tx.send(CoreEvent::KeyPress(evdev));
                    }
                }
            }
        }
        CallNextHookEx(HHOOK::default(), code, wparam, lparam)
    }

    fn run(stop_flag: Arc<AtomicBool>, tx: broadcast::Sender<CoreEvent>) {
        unsafe {
            *EVENT_TX.lock().unwrap() = Some(tx);

            let hook =
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::hook_proc), HINSTANCE(0), 0) {
                    Ok(h) => h,
                    Err(_) => {
                        *EVENT_TX.lock().unwrap() = None;
                        return;
                    }
                };

            let mut msg = MSG::default();
            while !stop_flag.load(Ordering::Relaxed) {
                msg = MSG::default();
                if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                } else {
                    if msg.message == WM_QUIT {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }

            let _ = UnhookWindowsHookEx(hook);
            *EVENT_TX.lock().unwrap() = None;
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

    fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.tx.subscribe()
    }
}
