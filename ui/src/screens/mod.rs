pub mod fan;
pub mod main;
pub mod settings;
pub mod start;

use core::fmt::Debug;
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};
use enum_dispatch::enum_dispatch;

use fan::FanScreen;
use main::MainScreen;
use rclite::Rc;
use settings::SettingsScreen;
use spin::RwLock;
use start::StartScreen;

#[allow(async_fn_in_trait)]
#[enum_dispatch(Screens<DT>)]
pub trait Screen<DT: DrawTarget<Error: Debug>> {
    async fn draw_init(&mut self, display: &mut DT);
    fn draw_static(&mut self, display: &mut DT);
}

#[allow(clippy::large_enum_variant)]
#[enum_dispatch]
pub enum Screens<DT: DrawTarget<Color = Rgb565, Error: Debug>> {
    Start(StartScreen<DT>),
    Main(Rc<RwLock<MainScreen<DT>>>),
    Fan(Rc<RwLock<FanScreen<DT>>>),
    Settings(Rc<RwLock<SettingsScreen<DT>>>),
}

#[derive(Debug, Clone)]
pub enum ItemSetting {
    Item(usize),
}
