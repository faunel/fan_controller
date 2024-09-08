use async_button::prelude::WaitPin;
use cortex_m::Peripherals as CortexPeripherals;
use display_interface_spi::SPIInterface;
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{models::ST7735s, options::{ColorOrder, Orientation, Rotation}, Builder, Display};
use monotonic::prelude::Mono;
use stm32f4::stm32f401::{Peripherals as STMPeripherals, ADC1, DMA2, EXTI, I2C2, SPI1, SYST, TIM1, TIM10, TIM11};
use stm32f4xx_hal::{
    adc::{config::{AdcConfig, Clock, Continuous, Dma, Resolution, SampleTime, Scan, Sequence}, Adc}, dma::{config::DmaConfig, PeripheralToMemory, Stream0, StreamsTuple, Transfer}, gpio::{gpioa, gpiob, gpioc, Analog, Edge, Output, Pin, Speed}, i2c::{self, I2c}, prelude::*, rcc::{Clocks, Rcc}, spi::{Mode, NoMiso, Phase, Polarity, Spi}, syscfg::SysCfg, timer::{self, CounterHz, Event, Timer}
};

type DMATransfer =
Transfer<Stream0<DMA2>, 0, Adc<ADC1>, PeripheralToMemory, &'static mut [u16; 2]>;

type TypeSPIInterface =
SPIInterface<ExclusiveDevice<Spi<SPI1>, Pin<'B', 2, Output>, embedded_hal_bus::spi::NoDelay>, Pin<'A', 15, Output>>;

type TypeDisplay =
Display<SPIInterface<ExclusiveDevice<Spi<SPI1>, Pin<'B', 2, Output>, embedded_hal_bus::spi::NoDelay>, Pin<'A', 15, Output>>, ST7735s, Pin<'B', 0, Output>>;

pub struct Init {
    pub gpioa: gpioa::Parts,
    pub gpiob: gpiob::Parts,
    pub gpioc: gpioc::Parts,
    syscfg: SysCfg,
    rcc: Rcc,
    adc: ADC1,
    tim_10: TIM10,
    syst: SYST,
    dma_2: DMA2,
    exti: EXTI,
    tim_11: TIM11,
    tim_1: TIM1,
    spi_1: SPI1,
    i2c2: I2C2
}

pub struct Gpio {
    gpioa: gpioa::Parts,
    gpiob: gpiob::Parts,
    gpioc: gpioc::Parts,

}

pub struct Spi1Pin {
    sck: Pin<'A', 5, stm32f4xx_hal::gpio::Alternate<5>>,
    mosi: Pin<'A', 7, stm32f4xx_hal::gpio::Alternate<5>>,
    miso: stm32f4xx_hal::gpio::NoPin,
    pub rst: Pin<'B', 0, Output>,
    cs: Pin<'B', 2, Output>,
    dc: Pin<'A', 15, Output>,
}

pub struct Button {
    pub btn_minus: WaitPin<Pin<'A', 10>>,
    pub btn_ok: WaitPin<Pin<'A', 0>>,
    pub btn_plus: WaitPin<Pin<'A', 9>>,
}

pub struct ExtiPin {
    pub fan1_rpm: Pin<'B', 15>,
    pub fan2_rpm: Pin<'B', 14>,
    pub fan3_rpm: Pin<'B', 13>,
    pub fan4_rpm: Pin<'B', 12>,
}

pub struct ExtiConfig {
  
}

pub struct AdcPin {
    fan1_adc: Pin<'A', 1, Analog>,
    fan2_adc: Pin<'A', 2, Analog>,
}

impl Init {
    pub fn new(device: STMPeripherals, core: CortexPeripherals) -> Self {
        // let gpioa = device.GPIOA.split();
        // let gpiob = device.GPIOB.split();
        // let gpioc = device.GPIOC.split();
        // let syscfg = device.SYSCFG.constrain();
        // let rcc = device.RCC.constrain();
        // let adc = device.ADC1;
        // let tim_10 = device.TIM10;


        Init { 
            gpioa: device.GPIOA.split(),
            gpiob: device.GPIOB.split(),
            gpioc: device.GPIOC.split(),
            syscfg: device.SYSCFG.constrain(),
            rcc: device.RCC.constrain(),
            adc: device.ADC1,
            tim_10: device.TIM10,
            syst: core.SYST,
            dma_2: device.DMA2,
            exti: device.EXTI,
            tim_11: device.TIM11,
            tim_1: device.TIM1,
            spi_1: device.SPI1,
            i2c2: device.I2C2,
        }
    }

    pub fn syscfg(self) -> SysCfg {
        self.syscfg
    }

    pub fn clocks(self) -> Clocks {
        self
            .rcc
            .cfgr
            .use_hse(25.MHz())
            .sysclk(100.MHz())
            .freeze()
    }

    pub fn gpio(self) -> Gpio {
        Gpio {
            gpioa: self.gpioa,
            gpiob: self.gpiob,
            gpioc: self.gpioc,
        }
    }

    pub fn spi1_pin(self) -> Spi1Pin {
        Spi1Pin {
            sck: self.gpioa.pa5.into_alternate().speed(Speed::VeryHigh),
            mosi: self.gpioa.pa7.into_alternate().speed(Speed::VeryHigh),
            miso:  NoMiso::new(),
            rst: self.gpiob.pb0.into_push_pull_output().speed(Speed::Medium),
            cs: self.gpiob.pb2.into_push_pull_output().speed(Speed::Medium),
            dc: self.gpioa.pa15.into_push_pull_output().speed(Speed::VeryHigh),
        }
    }

    pub fn delay(self, clocks: &Clocks) -> timer::Delay<TIM10, 1000000> {
        self.tim_10.delay_us(clocks)
    }

    pub fn mono_start(self, clocks: &Clocks) {
        Mono::start(self.syst, clocks.sysclk().to_Hz());
    }

    pub fn button(self) -> Button {
        // Button pin configuration
        let btn_minus = self.gpioa.pa10.into_pull_up_input();
        let btn_ok = self.gpioa.pa0.into_pull_up_input();
        let btn_plus = self.gpioa.pa9.into_pull_up_input();

        // Button async wrapper
        let btn_minus = WaitPin::new(btn_minus);
        let btn_ok = WaitPin::new(btn_ok);
        let btn_plus = WaitPin::new(btn_plus);

        Button {
            btn_minus,
            btn_ok,
            btn_plus,
        }
    }

    pub fn adc_pin(self) -> AdcPin {
        AdcPin { 
            fan1_adc: self.gpioa.pa1.into_analog(),
            fan2_adc: self.gpioa.pa2.into_analog(),
        }
    }

    pub fn adc_config(&self) -> AdcConfig {
        let adc_config: AdcConfig = AdcConfig::default()
            .dma(Dma::Continuous)
            .scan(Scan::Enabled)
            .clock(Clock::Pclk2_div_8)
            .continuous(Continuous::Continuous)
            .resolution(Resolution::Twelve);

        adc_config
        // ADC channel configuration
    }


    pub fn adc_channel_config(self, adc_config: AdcConfig) -> Adc<ADC1> {
        let mut adc: Adc<ADC1> = Adc::adc1(self.adc, true, adc_config);
        adc.configure_channel(&self.gpioa.pa1.into_analog(), Sequence::One, SampleTime::Cycles_480);
        adc.configure_channel(&self.gpioa.pa2.into_analog(), Sequence::Two, SampleTime::Cycles_480);
        adc
    }

    pub fn dma_config(&self) -> DmaConfig {
        let dma_config = DmaConfig::default()
            .transfer_complete_interrupt(true)
            .memory_increment(true)
            .double_buffer(false);
        dma_config
    }

    pub fn transfer(self, adc_channel_config: Adc<ADC1>, dma_config: DmaConfig, first_buffer: &'static mut [u16; 2]) -> DMATransfer {
        let dma = StreamsTuple::new(self.dma_2).0;

        Transfer::init_peripheral_to_memory(dma, adc_channel_config, first_buffer, None, dma_config)
    }

    pub fn exti_pin(self) -> ExtiPin {
        ExtiPin { 
            fan1_rpm: self.gpiob.pb15.into_pull_up_input(),
            fan2_rpm: self.gpiob.pb14.into_pull_up_input(),
            fan3_rpm: self.gpiob.pb13.into_pull_up_input(),
            fan4_rpm: self.gpiob.pb12.into_pull_up_input(),
        }
    }

    pub fn exti_config(&mut self, exti_pin: &mut ExtiPin) {
        exti_pin.fan1_rpm.make_interrupt_source(&mut self.syscfg);

        exti_pin.fan1_rpm.enable_interrupt(&mut self.exti);
        exti_pin.fan1_rpm.trigger_on_edge(&mut self.exti, Edge::Falling);
    }

    // Timer config. For frequency measurement of EXTI pin
    pub fn timer_11_config(self, clocks: &Clocks) -> CounterHz<TIM11> {
        let timer: Timer<TIM11> = Timer::new(self.tim_11, clocks);
        let mut counter_hz: CounterHz<TIM11> = timer.counter_hz();
        counter_hz.start(4.kHz()).unwrap();
        counter_hz.listen(Event::Update);
        counter_hz
    }

    pub fn timer_1_config(self, clocks: &Clocks) {
        let channels = timer::Channel1::new(self.gpioa.pa8);
        let timer = Timer::new(self.tim_1, &clocks);
        let mut pwm = timer.pwm_hz(channels, 14880.Hz());
        pwm.set_duty(timer::Channel::C1, pwm.get_max_duty() / 2);
        pwm.enable(timer::Channel::C1);
    }

    pub fn spi_interface(self, spi_pin: Spi1Pin, clocks: &Clocks) -> TypeSPIInterface {
        let mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };

        // SPI interface
        let spi = Spi::new(self.spi_1, (spi_pin.sck, spi_pin.miso, spi_pin.mosi), mode, 10.MHz(), clocks);
        let device = ExclusiveDevice::new_no_delay(spi, spi_pin.cs).unwrap();
        SPIInterface::new(device, spi_pin.dc)
    }

    pub fn display(self, interface: TypeSPIInterface, rst: Pin<'B', 0, Output>, delay: &mut timer::Delay<TIM10, 1000000>) -> TypeDisplay {
        Builder::new(ST7735s, interface)
        .orientation(Orientation {
            rotation: Rotation::Deg90,
            mirrored: false,
        })
            .color_order(ColorOrder::Rgb)
            .reset_pin(rst)
            .init(delay)
            .unwrap()
    }

    pub fn i2c(self, clocks: &Clocks) -> I2c<I2C2> {
        let scl = self.gpiob.pb10;
        let sda = self.gpiob.pb3.into_floating_input();
        I2c::new(self.i2c2, (scl, sda), i2c::Mode::standard(100.kHz()), clocks)
    }


}
