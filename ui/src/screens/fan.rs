use crate::{screens::Screen, BACKGROUND_COLOR};
use core::fmt::{Write, Debug};
#[allow(unused)]
use defmt::info;
use eeprom::eeprom::{FanNtc, SettingFan};
use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size, WebColors},
    primitives::{Line, PrimitiveStyle, Rectangle, StyledDrawable},
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
use ufmt::uwrite;

use crate::config::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

use super::ItemSetting;

impl Default for ItemSetting {
    fn default() -> Self {
        Self::Item(0)
    }
}

#[derive(Clone)]
pub struct FanScreen<DT> {
    pub fans: SettingFan,
    ntc_no: [FanNtc; 4],
    pub buffer: [u16; 8],
    pub fan_number: usize,
    pub item_setting: ItemSetting,
    prev_item_settings: usize,
    pub is_clear: bool,
    #[allow(unused)]
    prev: u8,
    _phantom: core::marker::PhantomData<DT>,
}

impl<DT: DrawTarget<Color = Rgb565, Error: Debug>> Screen<DT> for Rc<RwLock<FanScreen<DT>>> {
    async fn draw_init(&mut self, display: &mut DT) {
        if let Some(mut fan_screen) = self.try_write() {
            fan_screen.draw(display).await;
        }
    }

    fn draw_static(&mut self, display: &mut DT) {
        if let Some(setting_screen) = self.try_read() {
            if setting_screen.is_clear {
                if setting_screen.fan_number == 1 {
                    display.clear(BACKGROUND_COLOR).unwrap();
                    setting_screen.draw_grid(display);
                    setting_screen.draw_labels(display);
                } else {
                    //display.
                    let rectangle = Rectangle::new(Point { x: 0, y: 0 }, Size { width: 280, height: 19 });
                    rectangle.draw_styled(&PrimitiveStyle::with_fill(BACKGROUND_COLOR), display).unwrap();
                    setting_screen.draw_labels(display);
                }
            }
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565>> FanScreen<DT> 
where
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: Debug,
{
    #[must_use]
    pub fn new() -> Self {
        FanScreen {
            fans: Default::default(),
            ntc_no: Default::default(),
            buffer: [0; 8],
            fan_number: Default::default(),
            item_setting: Default::default(),
            prev_item_settings: 0,
            is_clear: Default::default(),
            prev: 0,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn set_fans(&mut self, fans: SettingFan) -> &mut Self {
        self.fans = fans;
        self
    }

    pub fn set_fan_number(&mut self, fan_number: usize) -> &mut Self {
        self.fan_number = fan_number;
        self
    }

    pub fn set_item_setting(&mut self, item_setting: ItemSetting) -> &mut Self {
        self.item_setting = item_setting;
        self
    }

    pub fn set_ntc_no(&mut self, ntc_no: [FanNtc; 4]) -> &mut Self {
        self.ntc_no = ntc_no;
        self
    }

    pub fn set_clear(&mut self, is_clear: bool) -> &mut Self {
        self.is_clear = is_clear;
        self
    }

    pub fn draw_grid(&self, display: &mut DT) {
        // Горизонтальні лінії
        let first_row_offset = 20;
        let second_row_offset = 40;
        let other_row_height = 50;
        let y = first_row_offset;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();

        let y = second_row_offset;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();

        let y = second_row_offset + other_row_height;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();

        let y = second_row_offset + other_row_height * 2;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();

        let y = second_row_offset + other_row_height * 3;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();

        // Вертикальні лінії
        let x = 140;
        Line::new(Point::new(x, first_row_offset), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display).unwrap();
    }

    pub fn draw_labels(&self, display: &mut DT) {
        let font = FontRenderer::new::<fonts::u8g2_font_profont22_mr>();

        let mut text: String<32> = String::new();
        uwrite!(&mut text, "SET. FAN {} -> NTC {}", self.fan_number, self.ntc_no[self.fan_number - 1].data).unwrap();
        font.render(text.as_str(), Point::new(30, 2), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_WHITE_SMOKE), display).unwrap();

        font.render("TEMP", Point::new(45, 22), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display).unwrap();

        font.render("PWM", Point::new(185, 22), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display).unwrap();
    }

    pub async fn draw(&mut self, display: &mut DT) {
        let mut style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(28, 47)) // digits are 10x20 pixels
            .digit_spacing(4) // 5px spacing between digits
            .segment_width(6) // 5px wide segments
            // .segment_color(Rgb565::new(10, 20, 31))  // active segments are green
            .segment_color(Rgb565::RED) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        let mut temp_colors = [Some(Rgb565::RED); 4];
        let mut pwm_colors = [Some(Rgb565::GREEN); 4];

        match self.item_setting {
            ItemSetting::Item(i) if (1..=8).contains(&i) => {
                let index = (i - 1) / 2;
                if i % 2 == 1 {
                    temp_colors[index] = Some(Rgb565::WHITE);
                } else {
                    pwm_colors[index] = Some(Rgb565::WHITE);
                }
            }
            _ => {}
        }

        // let mut y1 = 33;
        // let mut y2 = y1 + 6;
        // let mut y3 = y2 + 6;
        // Triangle::new(Point::new(0, y1), Point::new(6, y2), Point::new(0, y3))
        //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        //     .draw(display).unwrap();

        // y1 = 58;
        // y2 = y1 + 6;
        // y3 = y2 + 6;
        // Triangle::new(Point::new(0, y1), Point::new(6, y2), Point::new(0, y3))
        //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        //     .draw(display).unwrap();

        //     y1 = 83;
        //     y2 = y1 + 6;
        //     y3 = y2 + 6;
        // Triangle::new(Point::new(0, y1), Point::new(6, y2), Point::new(0, y3))
        //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        //     .draw(display).unwrap();

        //     y1 = 108;
        //     y2 = y1 + 6;
        //     y3 = y2 + 6;
        // Triangle::new(Point::new(0, y1), Point::new(6, y2), Point::new(0, y3))
        //     .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
        //     .draw(display).unwrap();

        let ItemSetting::Item(item) = self.item_setting;

        // Рядок перший. Температура
        let data = self.fans.thresold[0].temp.data;
        if self.buffer[0] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = temp_colors[0];
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(42, 88), style_segment).draw(display).unwrap();
            self.buffer[0] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок перший. ШІМ
        let data = self.fans.thresold[0].pwm.data;
        if self.buffer[1] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = pwm_colors[0];
            let mut text: String<3> = String::new();
            write!(text, "{:03}", data).unwrap();
            Text::new(&text, Point::new(164, 88), style_segment).draw(display).unwrap();
            self.buffer[1] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок другий. Температура
        let data = self.fans.thresold[1].temp.data;
        if self.buffer[2] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = temp_colors[1];
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(42, 138), style_segment).draw(display).unwrap();
            self.buffer[2] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок другий. ШІМ
        let data = self.fans.thresold[1].pwm.data;
        if self.buffer[3] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = pwm_colors[1];
            let mut text: String<3> = String::new();
            write!(text, "{:03}", data).unwrap();
            Text::new(&text, Point::new(164, 138), style_segment).draw(display).unwrap();
            self.buffer[3] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок третій. Температура
        let data = self.fans.thresold[2].temp.data;
        if self.buffer[4] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = temp_colors[2];
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(42, 188), style_segment).draw(display).unwrap();
            self.buffer[4] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок третій. ШІМ
        let data = self.fans.thresold[2].pwm.data;
        if self.buffer[5] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = pwm_colors[2];
            let mut text: String<3> = String::new();
            write!(text, "{:03}", data).unwrap();
            Text::new(&text, Point::new(164, 188), style_segment).draw(display).unwrap();
            self.buffer[5] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок четвертий. Температура
        let data = self.fans.thresold[3].temp.data;
        if self.buffer[6] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = temp_colors[3];
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(42, 238), style_segment).draw(display).unwrap();
            self.buffer[6] = data;
            Mono::delay(4.millis()).await;
        }

        // Рядок четвертий. ШІМ
        let data = self.fans.thresold[3].pwm.data;
        if self.buffer[7] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = pwm_colors[3];
            let mut text: String<3> = String::new();
            write!(text, "{:03}", data).unwrap();
            Text::new(&text, Point::new(164, 238), style_segment).draw(display).unwrap();
            self.buffer[7] = data;
            Mono::delay(4.millis()).await;
        }

        self.prev_item_settings = item;

    }
}

impl<DT: DrawTarget<Color = Rgb565>> Default for FanScreen<DT> 
where
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: Debug,
{
    fn default() -> Self {
        Self::new()
    }
}
