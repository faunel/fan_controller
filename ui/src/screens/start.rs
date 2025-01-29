use core::fmt::Debug;

use crate::screens::Screen;
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};

pub struct StartScreen<DT> {
    _phantom: core::marker::PhantomData<DT>,
}

impl<DT: DrawTarget<Color = Rgb565, Error: Debug>> Screen<DT> for StartScreen<DT> {
    async fn draw_init(&mut self, _display: &mut DT) {}
    fn draw_static(&mut self, _display: &mut DT) {}
}

impl<DT: DrawTarget<Color = Rgb565>> Default for StartScreen<DT> {
    fn default() -> Self {
        Self { _phantom: core::marker::PhantomData }
    }
}
