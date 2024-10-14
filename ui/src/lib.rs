#![no_std]

mod config;
pub mod screens;

pub use config::{Display, BACKGROUND_COLOR};
pub use screens::fan;
pub use screens::main;

#[derive(Debug)]
pub enum Menu {
    Main,
    Fan(usize),
    Settings(usize),
}
