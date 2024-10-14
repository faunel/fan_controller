use crate::screens::Screen;
use crate::{
    config::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
    BACKGROUND_COLOR,
};
use core::fmt::{Debug, Write};
#[allow(unused)]
use defmt::info;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size},
    primitives::{Line, PrimitiveStyle},
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

pub struct MainScreen<DT, E> {
    temp: [u16; 4],
    rpm: [u16; 4],
    buffer: [u16; 8],
    is_clear: bool,
    pub counter: u32,
    _phantom: core::marker::PhantomData<(DT, E)>,
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Screen<DT, E> for Rc<RwLock<MainScreen<DT, E>>> {
    async fn draw_init(&mut self, display: &mut DT) {
        if let Some(mut main_screen) = self.try_write() {
            main_screen.draw(display).await;
        }
    }

    fn draw_static(&mut self, display: &mut DT) {
        if let Some(main_screen) = self.try_read() {
            if main_screen.is_clear {
                display.clear(BACKGROUND_COLOR).unwrap();
                main_screen.draw_grid(display);
                main_screen.draw_labels(display);
            }
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> MainScreen<DT, E> {
    #[must_use]
    pub fn new() -> Self {
        MainScreen {
            temp: Default::default(),
            rpm: Default::default(),
            buffer: [0; 8],
            is_clear: false,
            counter: 0,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn set_temp(&mut self, temp: [u16; 4]) -> &mut Self {
        self.temp = temp;
        self
    }

    pub fn set_rpm(&mut self, temp: [u16; 4]) -> &mut Self {
        self.rpm = temp;
        self
    }

    pub fn set_counter(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

    pub fn set_clear(&mut self, is_clear: bool) -> &mut Self {
        self.is_clear = is_clear;
        self
    }

    pub fn get_clear(&self) -> bool {
        self.is_clear
    }

    pub fn draw_grid(&self, display: &mut DT) {
        // Горизонтальні лінії
        let first_row_height = 20;
        let other_row_height = 55;
        let y = first_row_height;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = first_row_height + other_row_height;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = first_row_height + other_row_height * 2;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = first_row_height + other_row_height * 3;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        // Вертикальні лінії
        let x = 66;
        Line::new(Point::new(x, 0), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let x = 138;
        Line::new(Point::new(x, 0), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();
    }

    pub fn draw_labels(&self, display: &mut DT) {
        let font = FontRenderer::new::<fonts::u8g2_font_profont22_mr>();

        font.render("FAN", Point::new(30, 2), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
            .unwrap();

        font.render("TEMP", Point::new(80, 2), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
            .unwrap();

        font.render("RPM", Point::new(186, 2), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
            .unwrap();
    }

    pub async fn draw(&mut self, display: &mut DT) {
        let mut style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(30, 52)) // digits are 10x20 pixels
            .digit_spacing(4) // 5px spacing between digits
            .segment_width(6) // 5px wide segments
            // .segment_color(Rgb565::new(10, 20, 31))  // active segments are green
            .segment_color(Rgb565::RED) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        // Рядок перший. Номер
        if self.is_clear {
            style_segment.segment_color = Some(Rgb565::WHITE);
            Text::new("1", Point::new(17, 73), style_segment).draw(display).unwrap();
            Mono::delay(4.millis()).await;
        }

        // Рядок перший. Температура
        let data = self.temp[0];
        if self.buffer[0] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::RED);
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(69, 73), style_segment).draw(display).unwrap();
            self.buffer[0] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок перший. Оберти
        let data = self.rpm[0];
        if self.buffer[1] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::BLUE);
            let mut text: String<4> = String::new();
            write!(text, "{:04}", data).unwrap();
            Text::new(&text, Point::new(143, 73), style_segment).draw(display).unwrap();
            self.buffer[1] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок другий. Номер
        if self.is_clear {
            style_segment.segment_color = Some(Rgb565::WHITE);
            Text::new("2", Point::new(17, 128), style_segment).draw(display).unwrap();
            Mono::delay(4.millis()).await;
        }

        // Рядок другий. Температура
        let data = self.temp[1];
        if self.buffer[2] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::RED);
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(69, 128), style_segment).draw(display).unwrap();
            self.buffer[2] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок другий. Оберти
        let data = self.rpm[1];
        if self.buffer[3] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::BLUE);
            let mut text: String<4> = String::new();
            write!(text, "{:04}", data).unwrap();
            Text::new(&text, Point::new(143, 128), style_segment).draw(display).unwrap();
            self.buffer[3] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок третій. Номер
        if self.is_clear {
            style_segment.segment_color = Some(Rgb565::WHITE);
            Text::new("3", Point::new(17, 183), style_segment).draw(display).unwrap();
            Mono::delay(4.millis()).await;
        }

        // Рядок третій. Температура
        let data = self.temp[2];
        if self.buffer[4] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::RED);
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(69, 183), style_segment).draw(display).unwrap();
            self.buffer[4] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок третій. Оберти
        let data = self.rpm[2];
        if self.buffer[5] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::BLUE);
            let mut text: String<4> = String::new();
            write!(text, "{:04}", data).unwrap();
            Text::new(&text, Point::new(143, 183), style_segment).draw(display).unwrap();
            self.buffer[5] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок четвертий. Номер
        if self.is_clear {
            style_segment.segment_color = Some(Rgb565::WHITE);
            Text::new("4", Point::new(17, 238), style_segment).draw(display).unwrap();
            Mono::delay(4.millis()).await;
        }

        // Рядок четвертий. Температура
        let data = self.temp[3];
        if self.buffer[6] != data || self.is_clear {
            style_segment.segment_color = Some(Rgb565::RED);
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(69, 238), style_segment).draw(display).unwrap();
            self.buffer[6] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок четвертий. Оберти
        let data = self.rpm[3];
        if self.buffer[7] != data || self.is_clear {
            style_segment.digit_size = Size::new(27, 48);
            style_segment.segment_color = Some(Rgb565::BLUE);
            let mut text: String<4> = String::new();
            write!(text, "{:04}", data).unwrap();
            Text::new(&text, Point::new(143, 235), style_segment).draw(display).unwrap();
            self.buffer[7] = data;
            Mono::delay(4.millis()).await;
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Default for MainScreen<DT, E> {
    fn default() -> Self {
        Self::new()
    }
}
