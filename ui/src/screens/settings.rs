use crate::screens::Screen;
use crate::BACKGROUND_COLOR;
use core::fmt::{Write, Debug};
#[allow(unused)]
use defmt::info;
use eeprom::eeprom::FanNtc;
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

use super::ItemSetting;
// use ufmt::uwrite;
// use defmt::{info, println};

pub struct SettingsScreen<DT> {
    backlight: u16,
    ntc_no: [FanNtc; 4],
    buffer: [u16; 8],
    is_clear: bool,
    item_setting: ItemSetting,
    prev_item_settings: usize,
    _phantom: core::marker::PhantomData<DT>,
}

impl<DT: DrawTarget<Color = Rgb565, Error: Debug>> Screen<DT> for Rc<RwLock<SettingsScreen<DT>>> {
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

impl<DT: DrawTarget<Color = Rgb565>> SettingsScreen<DT> 
where
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: Debug,
{
    pub fn new() -> Self {
        SettingsScreen {
            backlight: Default::default(),
            ntc_no: Default::default(),
            buffer: Default::default(),
            is_clear: false,
            item_setting: ItemSetting::default(),
            prev_item_settings: 0,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn set_item_setting(&mut self, item_setting: ItemSetting) -> &mut Self {
        self.item_setting = item_setting;
        self
    }

    pub fn set_backlight(&mut self, backlight: u16) -> &mut Self {
        self.backlight = backlight;
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
        let mut style_segment = SevenSegmentStyleBuilder::new()
            .digit_size(Size::new(17, 27)) // digits are 10x20 pixels
            .digit_spacing(4) // 5px spacing between digits
            .segment_width(4) // 5px wide segments
            .segment_color(Rgb565::GREEN) // active segments are green
            .inactive_segment_color(Rgb565::BLACK)
            .build();

        if self.is_clear {
            font.render("BACKLIGHT: ", Point::new(8, 27), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
            .unwrap();
            font.render("FAN 1 -> NTC", Point::new(8, 57), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
                .unwrap();
            font.render("FAN 2 -> NTC", Point::new(8, 87), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
                .unwrap();
            font.render("FAN 3 -> NTC", Point::new(8, 117), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
                .unwrap();
            font.render("FAN 4 -> NTC", Point::new(8, 147), VerticalPosition::Top, FontColor::Transparent(Rgb565::WHITE), display)
                .unwrap();
            Mono::delay(10.millis()).await;
        }

        let mut settings_colors = [Some(Rgb565::GREEN); 5];

        // self.item_setting = ItemSetting::Item(1);

        match self.item_setting {
            ItemSetting::Item(i) if (1..=5).contains(&i) => {
                settings_colors[i - 1] = Some(Rgb565::RED);
            }
            _ => {}
        }

        let ItemSetting::Item(item) = self.item_setting;

        let data = self.backlight;
        if self.buffer[0] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = settings_colors[0];
            let mut text: String<2> = String::new();
            write!(text, "{:02}", data).unwrap();
            Text::new(&text, Point::new(220, 50), style_segment).draw(display).unwrap();
            self.buffer[0] = data;
            Mono::delay(10.millis()).await;
        }

        let data = self.ntc_no[0].data;
        if self.buffer[1] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = settings_colors[1];
            let mut text: String<1> = String::new();
            write!(text, "{:01}", data).unwrap();
            Text::new(&text, Point::new(220, 80), style_segment).draw(display).unwrap();
            self.buffer[1] = data;
            Mono::delay(10.millis()).await;
        }

        let data = self.ntc_no[1].data;
        if self.buffer[2] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = settings_colors[2];
            let mut text: String<1> = String::new();
            write!(text, "{:01}", data).unwrap();
            Text::new(&text, Point::new(220, 110), style_segment).draw(display).unwrap();
            self.buffer[2] = data;
            Mono::delay(10.millis()).await;
        }

        let data = self.ntc_no[2].data;
        if self.buffer[3] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = settings_colors[3];
            let mut text: String<1> = String::new();
            write!(text, "{:01}", data).unwrap();
            Text::new(&text, Point::new(220, 140), style_segment).draw(display).unwrap();
            self.buffer[3] = data;
            Mono::delay(10.millis()).await;
        }

        let data = self.ntc_no[3].data;
        if self.buffer[4] != data || item != self.prev_item_settings || self.is_clear {
            style_segment.segment_color = settings_colors[4];
            let mut text: String<1> = String::new();
            write!(text, "{:01}", data).unwrap();
            Text::new(&text, Point::new(220, 170), style_segment).draw(display).unwrap();
            self.buffer[4] = data;
            Mono::delay(10.millis()).await;
        }

        self.prev_item_settings = item;
    }
}

impl<DT: DrawTarget<Color = Rgb565>> Default for SettingsScreen<DT> 
where
    DT: DrawTarget<Color = Rgb565>,
    DT::Error: Debug,
{
    fn default() -> Self {
        Self::new()
    }
}
