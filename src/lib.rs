pub mod core;
pub mod platform;
pub mod storage;
pub mod ui;

pub type Result<T> = std::result::Result<T, crate::platform::Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Platform error: {0}")]
    Platform(String),
}
