pub mod alpha_banks;
pub mod chord;
pub mod controller;
pub mod corpus;
pub mod independent;
pub mod inspector;
mod layers;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod model;
#[cfg(target_os = "linux")]
pub mod output;
pub mod render;
pub mod search;
pub mod typing;

pub type Result<T> = std::result::Result<T, String>;
