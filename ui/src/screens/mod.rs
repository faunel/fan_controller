pub mod main;
pub mod setting;
pub mod start;

use core::fmt::Debug;
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};
use enum_dispatch::enum_dispatch;
use main::MainScreen;
use setting::SettingScreen;
use start::StartScreen;

#[allow(async_fn_in_trait)]
#[enum_dispatch(Screens<DT, E>)]
pub trait Screen<DT: DrawTarget, E: Debug> {
    fn draw_init(&mut self, display: &mut DT);
    fn draw_static(&mut self, display: &mut DT);
}

#[allow(clippy::large_enum_variant)]
#[enum_dispatch]
pub enum Screens<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> {
    Start(StartScreen<DT, E>),
    Main(MainScreen<DT, E>),
    Setting(SettingScreen<DT, E>),
}
