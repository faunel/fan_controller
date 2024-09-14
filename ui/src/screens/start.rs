use core::fmt::Debug;
use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};

use crate::{screens::Screen, BACKGROUND_COLOR};

pub struct StartScreen<DT, E> {
    _phantom: core::marker::PhantomData<(DT, E)>,
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Screen<DT, E> for StartScreen<DT, E> {
    fn draw_init(&mut self, display: &mut DT) {
        display.clear(BACKGROUND_COLOR).unwrap();
    }

    fn draw_static(&mut self, _display: &mut DT) {}
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Default for StartScreen<DT, E> {
    fn default() -> Self {
        Self { _phantom: core::marker::PhantomData }
    }
}
