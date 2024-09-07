#![no_std]

mod config;
pub mod menu;
pub mod screens;

pub use config::{Display, BACKGROUND_COLOR};
pub use screens::main;
pub use screens::setting;
