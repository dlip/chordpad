pub mod chord;
pub mod controller;
pub mod corpus;
pub mod inspector;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod model;
#[cfg(target_os = "linux")]
pub mod output;
pub mod search;
pub mod typing;

pub type Result<T> = std::result::Result<T, String>;
