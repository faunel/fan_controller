// use display_interface_spi::SPIInterface;
use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use stm32f4xx_hal::{
    gpio::{Output, Pin},
    pac::SPI1,
    spi::Spi,
};

pub(crate) const DISPLAY_WIDTH: u32 = 280;
pub(crate) const DISPLAY_HEIGHT: u32 = 240;
pub const BACKGROUND_COLOR: Rgb565 = Rgb565::BLACK;

pub type Display<'a> = mipidsi::Display<
    SpiInterface<'a, ExclusiveDevice<Spi<SPI1>, Pin<'B', 12, Output>, NoDelay>, Pin<'B', 13, Output>>,
    ST7789,
    Pin<'B', 14, Output>,
>;
