use crate::{screens::Screen, BACKGROUND_COLOR};
use core::fmt::{Debug, Write};
use defmt::info;
use eeprom::eeprom::SettingFan;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Point, Primitive, RgbColor, Size, WebColors},
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
use ufmt::uwrite;

use crate::config::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

#[derive(Debug, Clone)]
pub enum ItemSetting {
    Item(usize),
}

impl Default for ItemSetting {
    fn default() -> Self {
        Self::Item(0)
    }
}

#[derive(Clone)]
pub struct SettingScreen<DT, E> {
    pub fans: SettingFan,
    pub fan_number: usize,
    pub item_setting: ItemSetting,
    pub is_clear: bool,
    prev: u8,
    _phantom: core::marker::PhantomData<(DT, E)>,
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Screen<DT, E> for Rc<RwLock<SettingScreen<DT, E>>> {
    async fn draw_init(&mut self, display: &mut DT) {
        if let Some(mut setting_screen) = self.try_write() {
            setting_screen.draw(display).await;
        }
    }

    fn draw_static(&mut self, display: &mut DT) {
        if let Some(setting_screen) = self.try_read() {
            if setting_screen.is_clear {
                display.clear(BACKGROUND_COLOR).unwrap();
                setting_screen.draw_grid(display);
                setting_screen.draw_labels(display);
            }
        }
    }
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> SettingScreen<DT, E> {
    #[must_use]
    pub fn new(fans: SettingFan, fan_number: usize, item_setting: ItemSetting, is_clear: bool) -> Self {
        SettingScreen {
            fans,
            fan_number,
            item_setting,
            is_clear,
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

    pub fn set_clear(&mut self, is_clear: bool) -> &mut Self {
        self.is_clear = is_clear;
        self
    }

    pub fn draw_grid(&self, display: &mut DT) {
        // Горизонтальні лінії
        let y = 13;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = 26;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = 26 + 25;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = 26 + 25 * 2;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        let y = 26 + 25 * 3;
        Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();

        // Вертикальні лінії
        let x = 79;
        Line::new(Point::new(x, 13), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
            .draw(display)
            .unwrap();
    }

    pub fn draw_labels(&self, display: &mut DT) {
        let font = FontRenderer::new::<fonts::u8g2_font_profont17_mr>();

        let mut text: String<32> = String::new();
        uwrite!(&mut text, "FAN {} SETTING", self.fan_number).unwrap();
        font.render(text.as_str(), Point::new(21, 0), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_WHITE_SMOKE), display)
            .unwrap();

        font.render("TEMP", Point::new(21, 13), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display)
            .unwrap();

        font.render("PWM", Point::new(106, 13), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display)
            .unwrap();
    }

    pub async fn draw(&mut self, display: &mut DT) {
        let mut style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(14, 22)) // digits are 10x20 pixels
            .digit_spacing(2) // 5px spacing between digits
            .segment_width(3) // 5px wide segments
            // .segment_color(Rgb565::new(10, 20, 31))  // active segments are green
            .segment_color(Rgb565::RED) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        info!("PREV: {}", self.prev);

        self.prev = self.prev.wrapping_add(1);

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

        // Рядок перший. Температура
        style_segment.segment_color = temp_colors[0];
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[0].temp.data).unwrap();
        Text::new(&text, Point::new(26, 49), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок перший. ШІМ
        style_segment.segment_color = pwm_colors[0];
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[0].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 49), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок другий. Температура
        style_segment.segment_color = temp_colors[1];
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[1].temp.data).unwrap();
        Text::new(&text, Point::new(26, 74), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок другий. ШІМ
        style_segment.segment_color = pwm_colors[1];
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[1].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 74), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок третій. Температура
        style_segment.segment_color = temp_colors[2];
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[2].temp.data).unwrap();
        Text::new(&text, Point::new(26, 99), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок третій. ШІМ
        style_segment.segment_color = pwm_colors[2];
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[2].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 99), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок четвертий. Температура
        style_segment.segment_color = temp_colors[3];
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[3].temp.data).unwrap();
        Text::new(&text, Point::new(26, 124), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;

        // Рядок четвертий. ШІМ
        style_segment.segment_color = pwm_colors[3];
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[3].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 124), style_segment).draw(display).unwrap();
        Mono::delay(5.millis()).await;
    }
}

// Обгортка навколо RefCell

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Default for SettingScreen<DT, E> {
    fn default() -> Self {
        Self {
            fans: Default::default(),
            fan_number: Default::default(),
            item_setting: Default::default(),
            is_clear: Default::default(),
            prev: 0,
            _phantom: Default::default(),
        }
    }
}
