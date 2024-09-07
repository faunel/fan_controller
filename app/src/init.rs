use cortex_m::Peripherals as CortexPeripherals;
use monotonic::prelude::Mono;
use stm32f4::stm32f401::{Peripherals as STMPeripherals, TIM10};
use stm32f4xx_hal::{
    gpio::{gpioa, gpiob, gpioc},
    prelude::*,
    rcc::Clocks,
    syscfg::SysCfg,
    timer,
};

pub struct Init {
    device: STMPeripherals,
    core: CortexPeripherals,
}

impl Init {
    pub fn new(device: STMPeripherals, core: CortexPeripherals) -> Self {
        Self { device, core }
    }

    pub fn syscfg(self) -> SysCfg {
        self.device.SYSCFG.constrain()
    }

    pub fn clocks(self) -> Clocks {
        self.device
            .RCC
            .constrain()
            .cfgr
            .use_hse(25.MHz())
            .sysclk(100.MHz())
            .freeze()
    }

    pub fn gpio(self) -> (gpioa::Parts, gpiob::Parts, gpioc::Parts) {
        (
            self.device.GPIOA.split(),
            self.device.GPIOB.split(),
            self.device.GPIOC.split(),
        )
    }

    pub fn delay(self, clocks: &Clocks) -> timer::Delay<TIM10, 1000000> {
        self.device.TIM10.delay_us(clocks)
    }

    pub fn mono_start(self, clocks: &Clocks) {
        Mono::start(self.core.SYST, clocks.sysclk().to_Hz());
    }

    pub fn adc(self) {}

    pub fn dma(self) {}

    pub fn spi(self) {}

    pub fn i2c(self) {}

    pub fn exti(self) {}

    pub fn display(self) {}
}
