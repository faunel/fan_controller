use display_interface_spi::SPIInterface;
use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::models::ST7735s;
use stm32f4xx_hal::{
    gpio::{Output, Pin},
    pac::SPI1,
    spi::Spi,
};

pub(crate) const DISPLAY_WIDTH: u32 = 160;
pub(crate) const DISPLAY_HEIGHT: u32 = 128;
pub const BACKGROUND_COLOR: Rgb565 = Rgb565::BLACK;

pub type Display = mipidsi::Display<
    SPIInterface<ExclusiveDevice<Spi<SPI1>, Pin<'B', 2, Output>, NoDelay>, Pin<'A', 15, Output>>,
    ST7735s,
    Pin<'B', 0, Output>,
>;
