#![no_std]

mod config;
pub mod screens;

pub use config::{Display, BACKGROUND_COLOR};
pub use screens::main;
pub use screens::setting;

#[derive(Debug)]
pub enum Menu {
    Main,
    Fan(usize),
}
