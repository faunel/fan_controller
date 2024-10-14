use crate::screens::Screen;
use crate::BACKGROUND_COLOR;
use core::fmt::{Debug, Write};
#[allow(unused)]
use defmt::info;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, RgbColor, Size},
    text::Text,
    Drawable,
};
use heapless::String;
use monotonic::prelude::*;
use rclite::Rc;
use spin::RwLock;
use u8g2_fonts::{
    fonts,
    types::{FontColor, VerticalPosition},
    FontRenderer,
};
// use ufmt::uwrite;
// use defmt::{info, println};

pub struct SettingsScreen<DT, E> {
    backlight: u16,
    buffer: [u16; 8],
    is_clear: bool,
    _phantom: core::marker::PhantomData<(DT, E)>,
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Screen<DT, E> for Rc<RwLock<SettingsScreen<DT, E>>> {
    async fn draw_init(&mut self, display: &mut DT) {
        if let Some(mut settings_screen) = self.try_write() {
            settings_screen.draw(display).await;
        }
    }

    fn draw_static(&mut self, display: &mut DT) {
        if let Some(main_screen) = self.try_read() {
            if main_screen.is_clear {
                display.clear(BACKGROUND_COLOR).unwrap();
                main_screen.draw_labels(display);
            }
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> SettingsScreen<DT, E> {
    #[must_use]
    pub fn new() -> Self {
        SettingsScreen {
            backlight: Default::default(),
            buffer: Default::default(),
            is_clear: false,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn set_backlight(&mut self, backlight: u16) -> &mut Self {
        self.backlight = backlight;
        self
    }

    pub fn set_clear(&mut self, is_clear: bool) -> &mut Self {
        self.is_clear = is_clear;
        self
    }

    pub fn get_clear(&self) -> bool {
        self.is_clear
    }

    pub fn draw_labels(&self, display: &mut DT) {
        let font = FontRenderer::new::<fonts::u8g2_font_profont22_mr>();

        font.render("SETTINGS", Point::new(90, 1), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
            .unwrap();
    }

    pub async fn draw(&mut self, display: &mut DT) {
        let font = FontRenderer::new::<fonts::u8g2_font_profont29_mr>();
        let style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(20, 37)) // digits are 10x20 pixels
            .digit_spacing(4) // 5px spacing between digits
            .segment_width(4) // 5px wide segments
            .segment_color(Rgb565::GREEN) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        if self.is_clear {
            font.render("BACKLIGHT: ", Point::new(8, 27), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
                .unwrap();
            Mono::delay(10.millis()).await;
        }

        let data = self.backlight;
        if self.buffer[0] != data || self.is_clear {
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(220, 55), style_segment).draw(display).unwrap();
            self.buffer[0] = data;
            Mono::delay(10.millis()).await;
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Default for SettingsScreen<DT, E> {
    fn default() -> Self {
        Self::new()
    }
}
