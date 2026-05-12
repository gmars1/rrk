use thiserror::Error;
use tokio::sync::broadcast;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// Events emitted by a platform listener.
#[derive(Debug, Clone, Copy)]
pub enum CoreEvent {
    KeyPress(u16),
}

/// Trait that all platform listeners must implement.
pub trait Listener: Send {
    fn start(&mut self) -> Result<(), Error>;
    fn stop(&mut self) -> Result<(), Error>;
    fn is_active(&self) -> bool;
    fn subscribe(&self) -> broadcast::Receiver<CoreEvent>;
}

/// Factory — creates the appropriate listener for the current OS.
pub fn create_listener() -> Result<Box<dyn Listener>, Error> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(crate::platform::linux::EvdevListener::new()?))
    }
    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(crate::platform::windows::WinListener::new()?))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(crate::platform::macos::MacListener::new()?))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Err(Error::Platform("Unsupported OS".into()))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Platform error: {0}")]
    Platform(String),
}
