use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use key_optimizer::core::keyboard::PhysicalKeyboard;
use key_optimizer::platform::{create_listener, CoreEvent};
use key_optimizer::storage;
use key_optimizer::ui::App;

fn load_builtin_layouts() -> Vec<PhysicalKeyboard> {
    let mut keyboards = Vec::new();
    let builtins = [
        include_str!("assets/layouts/qwerty.json"),
        include_str!("assets/layouts/colemak.json"),
    ];
    for json in &builtins {
        if let Ok(mut kb) = serde_json::from_str::<PhysicalKeyboard>(json) {
            kb.build_index();
            if !keyboards.iter().any(|k: &PhysicalKeyboard| k.id == kb.id) {
                keyboards.push(kb);
            }
        }
    }
    keyboards
}

fn build_mappings(kb: &PhysicalKeyboard) -> std::collections::HashMap<u16, char> {
    let mut map = std::collections::HashMap::new();
    for k in &kb.keys {
        if k.label.chars().count() != 1 {
            continue;
        }
        let ch = k.label.chars().next().unwrap();
        if ch != ' ' && !ch.is_alphabetic() {
            continue;
        }
        let sc = k.scan_codes.linux_evdev;
        if sc != 0 {
            map.insert(sc, ch);
        }
    }
    map
}

fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let keyboards = load_builtin_layouts();
    let kb = keyboards.first().expect("No keyboard layouts found");
    let sc_map = build_mappings(kb);

    let mut listener = create_listener()?;
    let mut rx = listener.subscribe();
    listener.start()?;

    let mut stats = storage::load_stats();
    let mut last_save = Instant::now();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
        eprintln!("[kbheat] Shutting down…");
    })?;

    eprintln!("[kbheat] Daemon started (pid {})", std::process::id());

    while running.load(Ordering::Relaxed) {
        while let Ok(CoreEvent::KeyPress(sc)) = rx.try_recv() {
            if let Some(&ch) = sc_map.get(&sc) {
                let lower = ch.to_lowercase().next().unwrap_or(ch);
                if lower != ' ' && !lower.is_alphabetic() {
                    continue;
                }
                stats.record_unigram(key_optimizer::core::id::CharId::new(lower));
            }
        }

        if last_save.elapsed() > Duration::from_secs(30) {
            storage::save_stats(&stats);
            last_save = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    storage::save_stats(&stats);
    eprintln!("[kbheat] Daemon stopped");
    Ok(())
}

fn run_ui() -> Result<(), Box<dyn std::error::Error>> {
    let keyboards = load_builtin_layouts();
    let stats = storage::load_stats();
    let mut listener = create_listener()?;
    let rx = listener.subscribe();
    listener.start()?;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_title("Keyboard Layout Optimizer"),
        ..Default::default()
    };

    eframe::run_native(
        "Keyboard Layout Optimizer",
        native_options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, stats, keyboards, rx, listener)))),
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--daemon") {
        run_daemon()
    } else {
        run_ui()
    }
}
