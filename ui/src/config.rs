use display_interface_spi::SPIInterface;
use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::models::ST7789;
use stm32f4xx_hal::{
    gpio::{Output, Pin},
    pac::SPI2,
    spi::Spi,
};

pub(crate) const DISPLAY_WIDTH: u32 = 280;
pub(crate) const DISPLAY_HEIGHT: u32 = 240;
pub const BACKGROUND_COLOR: Rgb565 = Rgb565::BLACK;

pub type Display = mipidsi::Display<
    SPIInterface<ExclusiveDevice<Spi<SPI2>, Pin<'A', 15, Output>, NoDelay>, Pin<'A', 10, Output>>,
    ST7789,
    Pin<'B', 12, Output>,
>;
