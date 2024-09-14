use crate::{screens::Screen, BACKGROUND_COLOR};
use core::fmt::{Debug, Write};
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

pub struct SettingScreen<DT, E> {
    fans: SettingFan,
    fan_number: usize,
    item_setting: ItemSetting,
    is_clear: bool,
    _phantom: core::marker::PhantomData<(DT, E)>,
}

impl<DT: DrawTarget<Color = Rgb565, Error = E>, E: Debug> Screen<DT, E> for SettingScreen<DT, E> {
    fn draw_init(&mut self, display: &mut DT) {
        self.draw(display);
    }

    fn draw_static(&mut self, display: &mut DT) {
        if self.is_clear {
            display.clear(BACKGROUND_COLOR).unwrap();
            self.draw_grid(display);
            self.draw_labels(display);
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
            _phantom: core::marker::PhantomData,
        }
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

        font.render("TEMP", Point::new(21, 13), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display).unwrap();

        font.render("PWM", Point::new(106, 13), VerticalPosition::Top, FontColor::Transparent(Rgb565::CSS_ORANGE), display).unwrap();
    }

    pub fn draw(&self, display: &mut DT) {
        let mut style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(14, 22)) // digits are 10x20 pixels
            .digit_spacing(2) // 5px spacing between digits
            .segment_width(3) // 5px wide segments
            // .segment_color(Rgb565::new(10, 20, 31))  // active segments are green
            .segment_color(Rgb565::RED) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        let mut fan1_temp_segment_color = Some(Rgb565::RED);
        let mut fan1_pwm_segment_color = Some(Rgb565::GREEN);
        let mut fan2_temp_segment_color = Some(Rgb565::RED);
        let mut fan2_pwm_segment_color = Some(Rgb565::GREEN);
        let mut fan3_temp_segment_color = Some(Rgb565::RED);
        let mut fan3_pwm_segment_color = Some(Rgb565::GREEN);
        let mut fan4_temp_segment_color = Some(Rgb565::RED);
        let mut fan4_pwm_segment_color = Some(Rgb565::GREEN);

        match self.item_setting {
            ItemSetting::Item(1) => fan1_temp_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(2) => fan1_pwm_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(3) => fan2_temp_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(4) => fan2_pwm_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(5) => fan3_temp_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(6) => fan3_pwm_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(7) => fan4_temp_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(8) => fan4_pwm_segment_color = Some(Rgb565::WHITE),
            ItemSetting::Item(_) => {}
        }

        // Рядок перший. Температура
        style_segment.segment_color = fan1_temp_segment_color;
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[0].temp.data).unwrap();
        Text::new(&text, Point::new(26, 49), style_segment).draw(display).unwrap();

        // Рядок перший. ШІМ
        style_segment.segment_color = fan1_pwm_segment_color;
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[0].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 49), style_segment).draw(display).unwrap();

        // Рядок другий. Температура
        style_segment.segment_color = fan2_temp_segment_color;
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[1].temp.data).unwrap();
        Text::new(&text, Point::new(26, 74), style_segment).draw(display).unwrap();

        // Рядок другий. ШІМ
        style_segment.segment_color = fan2_pwm_segment_color;
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[1].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 74), style_segment).draw(display).unwrap();

        // Рядок третій. Температура
        style_segment.segment_color = fan3_temp_segment_color;
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[2].temp.data).unwrap();
        Text::new(&text, Point::new(26, 99), style_segment).draw(display).unwrap();

        // Рядок третій. ШІМ
        style_segment.segment_color = fan3_pwm_segment_color;
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[2].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 99), style_segment).draw(display).unwrap();

        // Рядок четвертий. Температура
        style_segment.segment_color = fan4_temp_segment_color;
        let mut text: String<2> = String::new();
        write!(text, "{:02}", self.fans.thresold[3].temp.data).unwrap();
        Text::new(&text, Point::new(26, 124), style_segment).draw(display).unwrap();

        // Рядок четвертий. ШІМ
        style_segment.segment_color = fan4_pwm_segment_color;
        let mut text: String<3> = String::new();
        write!(text, "{:03}", self.fans.thresold[3].pwm.data).unwrap();
        Text::new(&text, Point::new(97, 124), style_segment).draw(display).unwrap();
    }
}
