#![no_main]
#![no_std]
// #![allow(unused)]

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI3, SPI4])]
mod app {
    use core::u16;


    use async_button::prelude::*;
    // #[allow(unused)]
    use defmt::info;
    // use display_interface_spi::SPIInterface;
    use eeprom::eeprom::{Settings, EEPROM};
    use embassy_futures::select::{select3, Either3};
    use embedded_alloc::LlffHeap as Heap;
    use embedded_hal::digital::InputPin;
    use embedded_hal_bus::spi::ExclusiveDevice;
    use measurements::{
        control::Control,
        measure::{AdcMeasure, Data, ImpulsesComplete, ImpulsesRaw, MeasureConfig},
        ADC_BUFFER,
    };
    use mipidsi::interface::SpiInterface;
    use mipidsi::{
        // interface::SpiError as DisplayError,
        // s::Error as DisplayError,
        options::{ColorOrder, Orientation, Rotation},
        Builder,
    };
    use mipidsi::{models::ST7789, options::ColorInversion};
    use monotonic::prelude::*;
    use ntc::Ntc;
    use rclite::Rc;
    extern crate alloc;
    use defmt_rtt as _;
    use panic_probe as _;
    use spin::rwlock::RwLock;
    #[allow(unused)]
    use stm32f4xx_hal::pac::DWT;
    use stm32f4xx_hal::{
        adc::{
            config::{AdcConfig, Clock, Continuous, Dma, Resolution, SampleTime, Scan, Sequence},
            Adc,
        },
        dma::{config::DmaConfig, PeripheralToMemory, Stream0, StreamsTuple, Transfer},
        gpio::{
            Edge, Pin, Speed,
        },
        hal::pwm::SetDutyCycle,
        i2c::{self, I2c},
        pac::{ADC1, DMA2, TIM1, TIM11, TIM2, TIM5, TIM9},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, NoMiso, Phase, Polarity, Spi},
        timer::{self, Event, Flag, Timer},
        watchdog::IndependentWatchdog,
    };
    use stm32f4xx_hal::{
        gpio::{Alternate, Analog, Input, Output, PinState, Pull},
        timer::{CaptureChannel, CaptureHzManager, CounterHz, FTimer}
    };
    use ui::Display;
    use ui::{
        fan::FanScreen,
        screens::{settings::SettingsScreen, ItemSetting},
    };
    use ui::{
        main::MainScreen,
        screens::{start::StartScreen, Screen, Screens},
        Menu,
    };

    // use stm32ral::{gpio, rcc, tim5 as T5};
    // use stm32ral::{modify_reg, read_reg, reset_reg, write_reg};

    type DMATransfer = Transfer<Stream0<DMA2>, 0, Adc<ADC1>, PeripheralToMemory, &'static mut [u16; ADC_BUFFER]>;

    #[shared]
    struct Shared {
        data: Data,
        transfer: DMATransfer,
        menu: Menu,
        settings: Settings,
        item_setting: ItemSetting,
        is_clear: bool,
        no_click_timer: Option<u8>,
        impulses_raw: ImpulsesRaw,
        impulses_complete: ImpulsesComplete,
        adc_buffer: Option<[u16; ADC_BUFFER]>,
        control: Control,
        // pwm_input_one: PwmInputOne,
        // pwm_input_two: PwmInputTwo,
        // duty_cycle_fan1: u8,
        // duty_cycle_fan2: u8,
    }

    #[local]
    struct Local {
        fan1_exti: Pin<'B', 1>,
        fan2_exti: Pin<'B', 0>,
        fan3_exti: Pin<'A', 7>,
        fan4_exti: Pin<'A', 6>,
        tim_1: (timer::PwmChannel<TIM1, 0>, timer::PwmChannel<TIM1, 1>),
        tim_9: timer::Counter<TIM9, 2000>,
        tim_11: CounterHz<TIM11>,
        buffer: Option<&'static mut [u16; ADC_BUFFER]>,
        btn_minus_async: WaitPin<Pin<'B', 2>>,
        btn_ok_async: WaitPin<Pin<'B', 14>>,
        btn_plus_async: WaitPin<Pin<'C', 13>>,
        adc: AdcMeasure,
        eeprom: EEPROM,
        iwdg: IndependentWatchdog,
        tim_2: timer::PwmChannel<TIM2, 0>,
        tim5: CaptureHzManager<TIM5>,
        tim5_ch1: timer::CaptureChannel<TIM5, 0>,
    }

    #[global_allocator]
    static HEAP: Heap = Heap::empty();

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        // Heap config
        {
            use core::mem::MaybeUninit;
            const HEAP_SIZE: usize = 512;
            static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
            #[allow(static_mut_refs)]
            unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
        }

        let dp = cx.device;
        let cp = cx.core;

        // Clock config
        let clocks = dp
            .RCC
            .constrain()
            .cfgr
            .use_hse(25.MHz())
            .require_pll48clk()
            .sysclk(100.MHz())
            .hclk(100.MHz())
            .pclk1(50.MHz())
            .pclk2(100.MHz())
            .freeze();

        // Monotonic timer
        Mono::start(cp.SYST, clocks.sysclk().to_Hz());

        // GPIO
        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();

        let mut syscfg = dp.SYSCFG.constrain();
        let mut exti = dp.EXTI;

        let mut delay = dp.TIM10.delay_us(&clocks);

        // cp.DCB.enable_trace();
        // cp.DWT.enable_cycle_counter();

        // let start = DWT::cycle_count();
        // info!("{}", DWT::cycle_count() - start);

        //---------------------------- Start GPIO config -----------------------
        // SPI1 pin config
        let sck = gpiob.pb13.into_alternate().speed(Speed::VeryHigh);
        let mosi = gpiob.pb15.into_alternate().speed(Speed::VeryHigh);
        let miso = NoMiso::new();
        let rst: Pin<'B', 12, Output> = Output::new(gpiob.pb12, PinState::Low).speed(Speed::Medium);
        let cs = Output::new(gpioa.pa15, PinState::Low).speed(Speed::Medium);
        let dc: Pin<'A', 10, Output> = Output::new(gpioa.pa10, PinState::Low).speed(Speed::VeryHigh);

        // I2C2 pin config
        let scl = Input::new(gpiob.pb10, Pull::Up);
        let sda = Input::new(gpiob.pb3, Pull::Up);

        // Button pin config
        let btn_minus = Input::new(gpiob.pb2, Pull::Up);
        let btn_ok: Pin<'B', 14> = Input::new(gpiob.pb14, Pull::Up);
        let btn_plus = Input::new(gpioc.pc13, Pull::Up);

        // Button async wrapper
        let btn_minus_async = WaitPin::new(btn_minus);
        let mut btn_ok_async: WaitPin<Pin<'B', 14>> = WaitPin::new(btn_ok);
        let btn_plus_async = WaitPin::new(btn_plus);

        // Buzzer pin
        let _buzzer: Pin<'B', 5, Output> = Output::new(gpiob.pb5, PinState::Low);

        // Backlight pin
        let backlight = gpioa.pa5;

        // Fan control PWM out pin
        let pwm4_ch1 = gpiob.pb6;
        let pwm4_ch2 = gpiob.pb7;
        let pwm4_ch3 = gpiob.pb8;
        let pwm4_ch4 = gpiob.pb9;
        // let m: Pin<'A', 0, stm32f4xx_hal::gpio::Alternate<2>> = gpioa.pa0.into_alternate();
        let pin_input_capture: Pin<'A', 0, Alternate<2>> = gpioa.pa0.into_alternate();

        // let pwm1_in = Input::new(gpioa.pa0, Pull::Down);
        let pwm2_in = Input::new(gpiob.pb4, Pull::Down);
        // let m = pwm1_in.make_interrupt_source(syscfg);

        // Fan emulator PWM out pin
        let fan_emulator_pwm1: Pin<'A', 8, Output> = Output::new(gpioa.pa8, PinState::Low);
        let fan_emulator_pwm2: Pin<'A', 9, Output> = Output::new(gpioa.pa9, PinState::Low);

        // EXTI pin config
        let mut fan1_exti = Input::new(gpiob.pb1, Pull::Up);
        let mut fan2_exti = Input::new(gpiob.pb0, Pull::Up);
        let mut fan3_exti = Input::new(gpioa.pa7, Pull::Up);
        let mut fan4_exti = Input::new(gpioa.pa6, Pull::Up);

        // ADC pin config
        let fan1_adc = Analog::new(gpioa.pa4);
        let fan2_adc = Analog::new(gpioa.pa3);
        let fan3_adc = Analog::new(gpioa.pa2);
        let fan4_adc = Analog::new(gpioa.pa1);
        //---------------------------- Finish GPIO config -----------------------

        //---------------------------- Start ADC config -----------------------
        // DMA stream
        let dma = StreamsTuple::new(dp.DMA2);

        // ADC config
        let adc_config = AdcConfig::default()
            .dma(Dma::Continuous)
            .scan(Scan::Enabled)
            .clock(Clock::Pclk2_div_8)
            .continuous(Continuous::Single)
            .resolution(Resolution::Twelve);

        // ADC channel config
        let mut adc = Adc::adc1(dp.ADC1, true, adc_config);
        adc.configure_channel(&fan1_adc, Sequence::One, SampleTime::Cycles_480);
        adc.configure_channel(&fan2_adc, Sequence::Two, SampleTime::Cycles_480);
        adc.configure_channel(&fan3_adc, Sequence::Three, SampleTime::Cycles_480);
        adc.configure_channel(&fan4_adc, Sequence::Four, SampleTime::Cycles_480);

        // ADC buffers
        let adc_buffer = [0; ADC_BUFFER];
        let first_buffer = cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap();
        let adc_dma_buffer = Some(cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap());

        // DMA config
        let dma_config = DmaConfig::default().transfer_complete_interrupt(true).memory_increment(true).double_buffer(false);

        // DMA transfer
        let transfer = Transfer::init_peripheral_to_memory(dma.0, adc, first_buffer, None, dma_config);
        //---------------------------- Finish ADC config -----------------------

        // Sowtware PWM
        // let sowtware_pwm1 = SowtwarePwm::new(pwm_fan_dummy1);
        // let sowtware_pwm2 = SowtwarePwm::new(pwm_fan_dummy2);

        //---------------------------- Start hardware interrupts -----------------------

        // EXTI pb15 config. Частота вентилятора
        fan1_exti.make_interrupt_source(&mut syscfg);
        fan1_exti.enable_interrupt(&mut exti);
        fan1_exti.trigger_on_edge(&mut exti, Edge::Falling);

        // EXTI pb14 config. Частота вентилятора
        fan2_exti.make_interrupt_source(&mut syscfg);
        fan2_exti.enable_interrupt(&mut exti);
        fan2_exti.trigger_on_edge(&mut exti, Edge::Falling);

        // EXTI pb13 config. Частота вентилятора
        fan3_exti.make_interrupt_source(&mut syscfg);
        fan3_exti.enable_interrupt(&mut exti);
        fan3_exti.trigger_on_edge(&mut exti, Edge::Falling);

        // EXTI pb12 config. Частота вентилятора
        fan4_exti.make_interrupt_source(&mut syscfg);
        fan4_exti.enable_interrupt(&mut exti);
        fan4_exti.trigger_on_edge(&mut exti, Edge::Falling);

        // TIM1. For dummy fan
        let timer = Timer::new(dp.TIM1, &clocks);
        let (_, (ch1, ch2, ..)) = timer.pwm_hz(300.Hz());
        let mut ch1 = ch1.with(fan_emulator_pwm1);
        let mut ch2 = ch2.with(fan_emulator_pwm2);
        ch1.set_duty_cycle_percent(50).unwrap();
        ch2.set_duty_cycle_percent(50).unwrap();
        ch1.enable();
        ch2.enable();
        let tim_1: (timer::PwmChannel<TIM1, 0>, timer::PwmChannel<TIM1, 1>) = (ch1, ch2);

        // TIM2. For backlight. PWM output config
        let timer = Timer::new(dp.TIM2, &clocks);
        let (_, (ch1, ..)) = timer.pwm_hz(167.Hz());
        let mut tim_2: timer::PwmChannel<TIM2, 0> = ch1.with(backlight);
        tim_2.set_duty(50);
        tim_2.enable();

        // TIM3. PWM input config
        let tim3 = Timer::new(dp.TIM3, &clocks);
        tim3.pwm_input(10.kHz(), pwm2_in);
        // let pwm_input_two = PwmInputTwo::new(tim3.pwm_input(10.kHz(), pwm2_in));

        // TIM4. PWM для вентиляторів. PWM output config
        let timer = Timer::new(dp.TIM4, &clocks);
        let (_, (ch1, ch2, ch3, ch4)) = timer.pwm_hz(25.kHz());
        let mut ch1 = ch1.with(pwm4_ch1);
        let mut ch2 = ch2.with(pwm4_ch2);
        let mut ch3 = ch3.with(pwm4_ch3);
        let mut ch4 = ch4.with(pwm4_ch4);
        ch1.set_polarity(timer::Polarity::ActiveLow);
        ch2.set_polarity(timer::Polarity::ActiveLow);
        ch1.set_duty(0);
        ch2.set_duty(0);
        ch3.set_duty(0);
        ch4.set_duty(0);
        ch1.enable();
        ch2.enable();
        ch3.enable();
        ch4.enable();
        let tim_4 = (ch1, ch2, ch3, ch4);

        // TIM5
        let (mut tim5, (tim5_ch1, ..)) = Timer::new(dp.TIM5, &clocks).capture_hz(1.MHz());
        let mut tim5_ch1: CaptureChannel<TIM5, 0> = tim5_ch1.with(pin_input_capture);
        // tim5_ch1.set_polarity(timer::Polarity::ActiveHigh);
        tim5_ch1.set_prescaler(timer::CapturePrescaler::Eight);
        // tim5_ch1.set_filter(timer::CaptureFilter::FckIntN8);

        tim5_ch1.enable();
        tim5.listen(Event::C1);

        // ---------------- За допомогою регістрів ------------
        // let mut tim5 = Timer::new(dp.TIM5, &clocks).counter_hz();
        // let ral_tim5 = T5::TIM5::take().unwrap();
        // unsafe { modify_reg!(rcc, RCC, APB1ENR, TIM5EN: Enabled) };
        // modify_reg!(T5, ral_tim5, CCMR1, CC1S: 0b01);
        // modify_reg!(T5, ral_tim5, CCER, CC1E: 1);
        // modify_reg!(T5, ral_tim5, PSC, PSC: 3);
        // modify_reg!(T5, ral_tim5, ARR, ARR: 0xFFFFFFFF);
        // modify_reg!(T5, ral_tim5, CR1, URS: 1);
        // write_reg!(T5, ral_tim5, EGR, UG: 1);
        // modify_reg!(T5, ral_tim5, CR1, URS: 0);
        // modify_reg!(T5, ral_tim5, CR1, CEN: 1);
        // modify_reg!(T5, ral_tim5, DIER, CC1IE: Enabled);
        // ---------------------------

        // // Читання значення захоплення
        // let capture_value = tim5.ccr1().read().ccr().bits();

        // let mut tim5 = Timer::new(dp.TIM5, &clocks).counter_hz();
        // tim5.listen(Event::C1);

        // tim5.

        // let (mng, (ch1, ..)) = tim5.pwm_hz(10.kHz());
        // let ch1 = ch1.with(pwm1_in);
        // mng.
        // let mut timer = Timer::new(dp.TIM5, &clocks);
        // let t = timer.counter_hz().configure(&clocks);

        // timer.start(1.MHz()).unwrap();
        // timer.set_master_mode(TIM1::c);
        // timer.listen(Event::C1);

        // let mut counter = timer.counter_hz();
        // counter.start(1.MHz()).unwrap();
        // counter.

        // t.start(1.MHz());
        // timer.listen(Event::C1);
        // timer.

        // TIM9. Для вимірювання частоти вентиляторів. Update interrupt config
        // Викликати раз на секунду
        let mut tim_9 = FTimer::new(dp.TIM9, &clocks).counter();
        // let mut tim_9 = timer.counter_hz();
        tim_9.start(1000.millis()).unwrap();
        tim_9.listen(Event::Update);

        // TIM11. Для запуску АЦП. Update interrupt config
        // Викликати не частіше 10ms
        let timer: Timer<TIM11> = Timer::new(dp.TIM11, &clocks);
        let mut tim_11: CounterHz<TIM11> = timer.counter_hz();
        tim_11.start(100.Hz()).unwrap();
        tim_11.listen(Event::Update);
        //---------------------------- Finish hardware interrupts -----------------------

        // SPI2 mode configuration
        let mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };

        // SPI interface
        let spi_buffer: &'static mut [u8; 512] = cortex_m::singleton!(: [u8; 512] = [0; 512]).unwrap();

        let spi = Spi::new(dp.SPI2, (sck, miso, mosi), mode, 25.MHz(), &clocks);
        let device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
        let interface = SpiInterface::new(device, dc, spi_buffer);

        // Display config
        let display: Display = Builder::new(ST7789, interface)
            .orientation(Orientation {
                rotation: Rotation::Deg90,
                mirrored: false,
            })
            .display_offset(0, 20)
            .color_order(ColorOrder::Rgb)
            .invert_colors(ColorInversion::Inverted)
            .reset_pin(rst)
            .display_size(240, 280)
            .init(&mut delay)
            .unwrap();

        let ntc = Ntc::default();
        let measure_config = MeasureConfig { temp_ema_window: 25, rpm_ema_window: 1 };
        let data = Data::new(ntc, measure_config);

        // Tasks
        eeprom_task::spawn(&mut btn_ok_async).ok();
        data_task::spawn().unwrap();
        // duty_cycle_task::spawn().unwrap();
        // software_pwm_fan1_task::spawn().unwrap();
        // software_pwm_fan2_task::spawn().unwrap();

        // Ці задачі повинні бути створені після задачі "eeprom_task"
        // Це пояснюється тим, що для цих задач структура settings повинна бути з даними, отриманими з EEPROM
        {
            display_menu_task::spawn(display).ok();
            button_task::spawn().unwrap();
            backlight_task::spawn().unwrap();
            pwm_fan_task::spawn().unwrap();
        }

        (
            Shared {
                data,
                transfer,
                menu: Menu::Main,
                settings: Settings::new(),
                item_setting: ItemSetting::Item(1),
                is_clear: true,
                no_click_timer: None,
                impulses_raw: ImpulsesRaw::new(),
                impulses_complete: ImpulsesComplete::new(),
                adc_buffer: Some(adc_buffer),
                control: Control::new(tim_4),
                // pwm_input_one,
                // pwm_input_two,
                // duty_cycle_fan1: 200,
                // duty_cycle_fan2: 200,
            },
            Local {
                fan1_exti,
                fan2_exti,
                fan3_exti,
                fan4_exti,
                tim_1,
                tim_9,
                tim_11,
                buffer: adc_dma_buffer,
                btn_minus_async,
                btn_ok_async,
                btn_plus_async,
                adc: AdcMeasure::new(),
                eeprom: EEPROM::new(I2c::new(dp.I2C2, (scl, sda), i2c::Mode::standard(100.kHz()), &clocks)),
                iwdg: IndependentWatchdog::new(dp.IWDG),
                tim_2,
                tim5,
                tim5_ch1,
            },
        )
    }

    // #[task(local = [sowtware_pwm1], shared = [duty_cycle_fan1], priority = 2)]
    // async fn software_pwm_fan1_task(mut cx: software_pwm_fan1_task::Context) {
    //     loop {
    //         let duty_cycle_fan1 = cx.shared.duty_cycle_fan1.lock(|duty_cycle_fan1| *duty_cycle_fan1);

    //         let (freq, duty_cycle) = if duty_cycle_fan1 < 3 {
    //             (1, 0)
    //         } else if duty_cycle_fan1 > 90 {
    //             (200, 20)
    //         } else {
    //             (duty_cycle_fan1 * 2, 20)
    //         };
    //         cx.local.sowtware_pwm1.pwm_hz(freq, duty_cycle).await;
    //     }
    // }

    // #[task(local = [sowtware_pwm2], shared = [duty_cycle_fan2], priority = 2)]
    // async fn software_pwm_fan2_task(mut cx: software_pwm_fan2_task::Context) {
    //     loop {
    //         let duty_cycle_fan1 = cx.shared.duty_cycle_fan2.lock(|duty_cycle_fan1| *duty_cycle_fan1);

    //         if duty_cycle_fan1 < 3 {
    //             cx.local.sowtware_pwm2.pwm_hz(1, 0).await;
    //         } else if duty_cycle_fan1 > 90 {
    //             cx.local.sowtware_pwm2.pwm_hz(200, 20).await;
    //         } else {
    //             cx.local.sowtware_pwm2.pwm_hz(duty_cycle_fan1 * 2, 20).await;
    //         }
    //     }
    // }

    // #[task(shared = [pwm_input_one, pwm_input_two, duty_cycle_fan1, duty_cycle_fan2], priority = 2)]
    // async fn duty_cycle_task(mut cx: duty_cycle_task::Context) {
    //     loop {
    //         // Отримання duty cycle для TIM5
    //         (&mut cx.shared.pwm_input_one, &mut cx.shared.duty_cycle_fan1).lock(|pwm_input_one, duty_cycle_fan1| {
    //             *duty_cycle_fan1 = pwm_input_one.get_duty_cycle();
    //         });

    //         // Отримання duty cycle для TIM3
    //         (&mut cx.shared.pwm_input_two, &mut cx.shared.duty_cycle_fan2).lock(|pwm_input_two, duty_cycle_fan2| {
    //             *duty_cycle_fan2 = pwm_input_two.get_duty_cycle();
    //         });

    //         Mono::delay(100.millis()).await;
    //     }
    // }

    // #[task(shared = [eeprom, settings], priority = 2)]
    // async fn default_settings_task(mut cx: default_settings_task::Context, btn_ok_async: &mut WaitPin<Pin<'B', 14>>) {
    //     if btn_ok_async.is_low().unwrap_or(false) {
    //         Mono::delay(2000.millis()).await;
    //         if btn_ok_async.is_low().unwrap_or(false) {
    //             let mut s = cx.shared.settings.lock(move |settings| settings.clone());
    //             cx.local.eeprom.default_settings(&mut s).await;
    //             cx.shared.settings.lock(|settings| {
    //                 *settings = s;
    //             });
    //         }
    //     }
    // }

    // Software task
    // Завантажує всі налаштування.
    // Зберігає налаштування при відсутності натискань кнопок за певний період при умові зміни будь якого параметру
    #[task(local = [eeprom], shared = [no_click_timer, settings, menu, is_clear, item_setting], priority = 2)]
    async fn eeprom_task(mut cx: eeprom_task::Context, btn_ok_async: &mut WaitPin<Pin<'B', 14>>) {
        if btn_ok_async.is_low().unwrap_or(false) {
            Mono::delay(2000.millis()).await;
            if btn_ok_async.is_low().unwrap_or(false) {
                let mut s = cx.shared.settings.lock(move |settings| settings.clone());
                cx.local.eeprom.default_settings(&mut s).await;
                cx.shared.settings.lock(|settings| {
                    *settings = s;
                });
            }
        }

        let mut s = cx.shared.settings.lock(move |settings| settings.clone());
        cx.local.eeprom.load_settings(&mut s).await;
        cx.shared.settings.lock(|settings| {
            *settings = s;
        });

        info!("load settings");

        loop {
            let s = cx.shared.no_click_timer.lock(|no_click_timer| {
                if let Some(t) = no_click_timer {
                    *t = t.saturating_sub(1);

                    if *t == 0 {
                        *no_click_timer = None;

                        (&mut cx.shared.menu, &mut cx.shared.is_clear, &mut cx.shared.item_setting).lock(|menu, is_clear, item_setting| {
                            match *menu {
                                Menu::Main => {}
                                _ => {
                                    *menu = Menu::Main;
                                    *is_clear = true;
                                    *item_setting = ItemSetting::Item(1);
                                }
                            }
                        });
                        return Some(cx.shared.settings.lock(|settings| settings.clone()));
                    }
                }
                None
            });

            if let Some(mut s) = s {
                cx.local.eeprom.save_all(&mut s).await;
            }

            Mono::delay(1000.millis()).await;
        }
    }

    // Software task
    // Для відображення даних на дисплеї
    #[task(shared = [menu, data, settings, item_setting, is_clear], priority = 1)]
    async fn display_menu_task(cx: display_menu_task::Context, mut display: Display<'_>) {
        let main_screen = Rc::new(RwLock::new(MainScreen::default()));
        let fan_screen = Rc::new(RwLock::new(FanScreen::default()));
        let settings_screen = Rc::new(RwLock::new(SettingsScreen::default()));

        // let display = unsafe { cx.shared.display.lock(|d| &mut *d.get()) };
        // let display = cx.local.display;
        let mut shared = cx.shared;

        let mut screen: Screens<Display> = StartScreen::default().into();

        loop {
            // info!("HEAP free: {}", HEAP.free());
            // info!("HEAP used: {}", HEAP.used()); // Output -> HEAP used: 112
            (&mut shared.menu, &mut shared.is_clear).lock(|menu, is_clear| {
                match menu {
                    Menu::Main => {
                        (&mut shared.data, &mut shared.settings).lock(|data, settings| {
                            if let Some(mut main_screen) = main_screen.try_write() {
                                main_screen
                                    .set_clear(*is_clear)
                                    .set_temp(*data.get_temp())
                                    .set_rpm(*data.get_rpm())
                                    .set_thresold(*data.get_thresold())
                                    .set_ntc_no(settings.ntc_no.clone());
                            }
                        });
                        screen = Screens::Main(Rc::clone(&main_screen));
                    }
                    Menu::Fan(fan) => {
                        (&mut shared.settings, &mut shared.item_setting).lock(|settings, item_setting| {
                            if let Some(mut fan_screen) = fan_screen.try_write() {
                                fan_screen
                                    .set_fans(settings.fans[*fan - 1].clone())
                                    .set_fan_number(*fan)
                                    .set_ntc_no(settings.ntc_no.clone())
                                    .set_item_setting(item_setting.clone())
                                    .set_clear(*is_clear);
                            }
                            screen = Screens::Fan(Rc::clone(&fan_screen));
                        });
                    }
                    Menu::Settings => {
                        (&mut shared.settings, &mut shared.item_setting).lock(|settings, item_setting| {
                            if let Some(mut settings_screen) = settings_screen.try_write() {
                                settings_screen
                                    .set_backlight(settings.backlight.data)
                                    .set_item_setting(item_setting.clone())
                                    .set_ntc_no(settings.ntc_no.clone())
                                    .set_clear(*is_clear);
                            }
                            screen = Screens::Settings(Rc::clone(&settings_screen));
                        });
                    }
                }
                *is_clear = false;
            });

            screen.draw_static(&mut display);
            screen.draw_init(&mut display).await;
          
            Mono::delay(20.millis()).await;
        }
    }

    // Software task
    // Обробник натисання кнопок і зміна параметрів налаштувань
    #[task(local = [btn_minus_async, btn_plus_async, btn_ok_async], shared = [no_click_timer, settings, item_setting, menu, is_clear], priority = 2)]
    async fn button_task(mut cx: button_task::Context) {
        // Button configuration
        let button_config =
            ButtonConfig::new(MyDuration::millis(20), MyDuration::millis(1), MyDuration::millis(500), ButtonMode::PullUp, 70);

        let mut btn_minus = Button::new(cx.local.btn_minus_async, button_config);
        let mut btn_ok = Button::new(cx.local.btn_ok_async, button_config);
        let mut btn_plus = Button::new(cx.local.btn_plus_async, button_config);

        let mut prev_pressed_minus_plus = false;
        loop {
            let select = select3(btn_minus.update(), btn_ok.update(), btn_plus.update()).await;

            let is_pressed_minus_plus = btn_minus.is_pin_pressed() && btn_plus.is_pin_pressed();
            if is_pressed_minus_plus {
                Mono::delay(500.millis()).await;
                if is_pressed_minus_plus {
                    (&mut cx.shared.menu, &mut cx.shared.is_clear, &mut cx.shared.item_setting).lock(|menu, is_clear, item_setting| {
                        match *menu {
                            Menu::Settings => {}
                            _ => {
                                *is_clear = true;
                                *menu = Menu::Settings;
                                *item_setting = ItemSetting::Item(1);
                            }
                        }
                    });
                    prev_pressed_minus_plus = true;
                    continue;
                }
            }

            // match select {
            //     Either3::First(_) => info!("minus"),
            //     Either3::Second(_) => info!("ok"),
            //     Either3::Third(_) => info!("plus"),
            // }

            cx.shared.no_click_timer.lock(|no_click_timer| *no_click_timer = Some(10));

            (&mut cx.shared.is_clear, &mut cx.shared.menu, &mut cx.shared.item_setting, &mut cx.shared.settings).lock(
                |is_clear, menu, item_setting, settings: &mut Settings| match select {
                    Either3::First(minus) => match minus {
                        ButtonEvent::ShortPress(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.decrement_logic(fan, item),
                            },
                            Menu::Main => {}
                            Menu::Settings => {
                                let ItemSetting::Item(item) = *item_setting;

                                if item == 1 {
                                    if settings.backlight.data > 0 && !prev_pressed_minus_plus {
                                        settings.backlight.data -= 1;
                                    }
                                } else {
                                    if settings.ntc_no[item - 2].data > 1 {
                                        settings.ntc_no[item - 2].data -= 1;
                                    }
                                }

                                prev_pressed_minus_plus = false;
                            }
                        },
                        ButtonEvent::LongPress => {}
                        ButtonEvent::LongPressDuration(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.decrement_logic(fan, item),
                            },
                            Menu::Main => {}
                            Menu::Settings => {
                                if settings.backlight.data > 0 {
                                    settings.backlight.data -= 1;
                                }
                            }
                        },
                    },
                    Either3::Second(ok) => match ok {
                        ButtonEvent::ShortPress(_) => {
                            let ItemSetting::Item(mut item) = *item_setting;
                            item += 1;
                            match menu {
                                Menu::Fan(_) => {
                                    if item > 8 {
                                        item = 1
                                    }
                                }
                                Menu::Settings => {
                                    if item > 5 {
                                        item = 1
                                    }
                                }
                                _ => {}
                            }
                            *item_setting = ItemSetting::Item(item);
                        }
                        ButtonEvent::LongPress => match menu {
                            Menu::Main => {
                                *is_clear = true;
                                *menu = Menu::Fan(1);
                                *item_setting = ItemSetting::Item(1);
                            }
                            Menu::Fan(mut fan) => {
                                *is_clear = true;
                                *item_setting = ItemSetting::Item(1);
                                if fan < 4 {
                                    fan += 1;
                                    *menu = Menu::Fan(fan);
                                } else {
                                    *menu = Menu::Main;
                                }
                            }
                            Menu::Settings => {
                                *is_clear = true;
                                *menu = Menu::Main;
                                cx.shared.no_click_timer.lock(|no_click_timer| *no_click_timer = Some(0));
                            }
                        },
                        ButtonEvent::LongPressDuration(_) => {}
                    },
                    Either3::Third(plus) => match plus {
                        ButtonEvent::ShortPress(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.increment_logic(fan, item),
                            },
                            Menu::Main => {}
                            Menu::Settings => {
                                let ItemSetting::Item(item) = *item_setting;

                                if item == 1 {
                                    if settings.backlight.data < 10 && !prev_pressed_minus_plus {
                                        settings.backlight.data += 1
                                    }
                                } else {
                                    if settings.ntc_no[item - 2].data < 4 {
                                        settings.ntc_no[item - 2].data += 1;
                                    }
                                }

                                prev_pressed_minus_plus = false;
                            }
                        },
                        ButtonEvent::LongPress => {}
                        ButtonEvent::LongPressDuration(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.increment_logic(fan, item),
                            },
                            Menu::Main => {}
                            Menu::Settings => {
                                if settings.backlight.data < 10 {
                                    settings.backlight.data += 1;
                                }
                            }
                        },
                    },
                },
            );
        }
    }

    // Sowtware task
    // Управління підсвіткою
    #[task(local = [tim_2], shared = [settings], priority = 2)]
    async fn backlight_task(mut cx: backlight_task::Context) {
        loop {
            Mono::delay(100.millis()).await;
            cx.shared.settings.lock(|settings| {
                let mut percent = settings.backlight.data as u8;
                percent = if percent > 0 { percent * 10 } else { 1 };
                cx.local.tim_2.set_duty_cycle_percent(percent).unwrap();
            });
        }
    }

    // Sowtware task
    // Управління логікою на основі виміряних даних
    #[task(shared = [control, data, settings], priority = 2)]
    async fn pwm_fan_task(mut cx: pwm_fan_task::Context) {
        loop {
            (&mut cx.shared.data, &mut cx.shared.settings, &mut cx.shared.control).lock(|data, settings, control| {
                // let temp = data.get_temp();
                control.run(settings, data);
            });
            Mono::delay(100.millis()).await;
        }
    }

    #[task(local = [tim_1], shared = [control, data], priority = 2)]
    async fn check_fan(mut cx: check_fan::Context) {
        loop {
            let (duty, rpm) = (&mut cx.shared.data, &mut cx.shared.control).lock(|data, control| {
                ([control.get_duty_cycle_percent_ch1(), control.get_duty_cycle_percent_ch2()], [data.get_rpm()[0], data.get_rpm()[1]])
            });
            Mono::delay(5.secs()).await;
            if duty[0] >= 20 && rpm[0] < 200 {
                cx.local.tim_1.0.set_duty_cycle_fully_off().unwrap();
            }
            if duty[1] >= 20 && rpm[1] < 200 {
                cx.local.tim_1.1.set_duty_cycle_fully_off().unwrap();
            }
        }
    }

    // Sowtware task
    // Запис виміряних даних в структуру
    #[task(local = [adc], shared = [adc_buffer, impulses_complete, data], priority = 2)]
    async fn data_task(mut cx: data_task::Context) {
        loop {
            let adc_buffer = cx.shared.adc_buffer.lock(|adc_buffer| adc_buffer.take());
            if let Some(buffer) = &adc_buffer {
                let adc_values: &[u16; 4] = cx.local.adc.split_channels(buffer);
                cx.shared.data.lock(|data| data.set_temp(adc_values));
            }
            (&mut cx.shared.data, &mut cx.shared.impulses_complete).lock(|data, impulses_complete| data.set_rpm(impulses_complete));
            Mono::delay(20.millis()).await;
        }
    }

    // Hardware task
    // TIM3. Отримання Duty cycle
    // #[task(binds = TIM3, shared = [pwm_input_two], priority = 3)]
    // fn tim_3(mut cx: tim_3::Context) {
    //     cx.shared.pwm_input_two.lock(|pwm_input_two| {
    //         pwm_input_two.set_dirty_duty_cycle();
    //         pwm_input_two.timer.clear_all_flags();
    //     });
    // }

    // Hardware task
    // TIM5. Отримання Duty cycle
    #[task(binds = TIM5, local = [tim5, tim5_ch1, prev_capture: u32 = 0, overflow_count: u32 = 0], priority = 3)]
    fn tim_5(cx: tim_5::Context) {
        if cx.local.tim5.flags().contains(Flag::C1) {
            let timer_clock = cx.local.tim5.get_timer_clock();
            let max_auto_reload = cx.local.tim5.get_max_auto_reload();
            let current_capture = cx.local.tim5_ch1.get_capture();

            // let delta = if current_capture >= *cx.local.prev_capture {
            //     current_capture as u32 - *cx.local.prev_capture as u32 + *cx.local.overflow_count * u16::MAX as u32
            // } else {
            //     (u16::MAX as u32 - *cx.local.prev_capture as u32) + current_capture as u32 + (*cx.local.overflow_count - 1) * u16::MAX as u32
            // };

            let delta = if current_capture >= *cx.local.prev_capture {
                current_capture - *cx.local.prev_capture
            } else {
                (max_auto_reload - *cx.local.prev_capture) + current_capture
            };

            let mut freq = timer_clock as f32 / delta as f32;

            freq *= 8.0;

            info!(
                "freq: {}, diff: {}, timer_clock: {}, prev_capture: {}, current_capture: {}",
                freq, delta, timer_clock, *cx.local.prev_capture, current_capture
            );

            *cx.local.prev_capture = current_capture;
            *cx.local.overflow_count = 0;
            cx.local.tim5.clear_flags(Flag::C1);
        }
    }

    // Hardware task
    // TIM9. Для вимірювання частоти вентиляторів.
    #[task(binds = TIM1_BRK_TIM9, local = [tim_9, round: u8 = 0], shared = [impulses_raw, impulses_complete], priority = 3)]
    fn tim_9(mut cx: tim_9::Context) {
        if cx.local.tim_9.flags().contains(Flag::Update) {
            cx.local.tim_9.clear_flags(Flag::Update);
        }

        let samples: u8 = 4;

        if *cx.local.round == samples {
            *cx.local.round = 0;

            cx.shared.impulses_raw.lock(|impulses_raw| {
                for (ind, fan) in impulses_raw.iter_mut().enumerate() {
                    cx.shared.impulses_complete.lock(|impulses_complete| {
                        let fan_result = f32::from(*fan) / f32::from(samples);
                        impulses_complete.set(ind, &fan_result)
                    });
                    *fan = 0;
                }
            });
        }

        *cx.local.round += 1;
    }

    // Hardware task
    // TIM11. Для запуску АЦП
    #[task(binds = TIM1_TRG_COM_TIM11, local = [tim_11], shared = [transfer], priority = 3)]
    fn tim_11(mut cx: tim_11::Context) {
        if cx.local.tim_11.flags().contains(Flag::Update) {
            cx.local.tim_11.clear_flags(Flag::Update);
        }

        cx.shared.transfer.lock(|transfer| {
            transfer.start(|adc| adc.start_conversion());
        });
    }

    #[task(binds = EXTI1, local = [fan1_exti, duration: u32 = 0], shared = [impulses_raw], priority = 4)]
    fn exti_1(mut cx: exti_1::Context) {
        cx.shared.impulses_raw.lock(|impulses_raw| {
            if cx.local.fan1_exti.check_interrupt() {
                impulses_raw.add_raw(0);
                cx.local.fan1_exti.clear_interrupt_pending_bit();
            }
        });
        // let now_1: fugit::Instant<u32, 1, 100_000> = Mono::now();
        // now_1.ticks();

        // let now_2 = Mono::now();
        // let m = now_2.checked_duration_since(now_1);
    }

    #[task(binds = EXTI0, local = [fan2_exti], shared = [impulses_raw], priority = 4)]
    fn exti_0(mut cx: exti_0::Context) {
        cx.shared.impulses_raw.lock(|impulses_raw| {
            if cx.local.fan2_exti.check_interrupt() {
                impulses_raw.add_raw(1);
                cx.local.fan2_exti.clear_interrupt_pending_bit();
            }
        });
    }

    // Hardware task
    // Частота вентилятора. Викликається при надходженні сигналу на pin
    #[task(binds = EXTI9_5, local = [fan3_exti, fan4_exti], shared = [impulses_raw], priority = 4)]
    fn exti_15(mut cx: exti_15::Context) {
        cx.shared.impulses_raw.lock(|impulses_raw| {
            if cx.local.fan3_exti.check_interrupt() {
                impulses_raw.add_raw(2);
                cx.local.fan3_exti.clear_interrupt_pending_bit();
            }
            if cx.local.fan4_exti.check_interrupt() {
                impulses_raw.add_raw(3);
                cx.local.fan4_exti.clear_interrupt_pending_bit();
            }
        });
    }

    // Hardware task
    // DMA. Викликається коли дані готові
    #[task(binds = DMA2_STREAM0, shared = [transfer, adc_buffer], local = [buffer], priority = 5)]
    fn dma(mut cx: dma::Context) {
        let buffer = cx.shared.transfer.lock(|transfer| {
            let (buffer, _) = transfer.next_transfer(cx.local.buffer.take().unwrap()).unwrap();
            buffer
        });

        cx.shared.adc_buffer.lock(|adc_buffer| {
            *adc_buffer = Some(*buffer);
        });

        *cx.local.buffer = Some(buffer);
    }

    #[idle(local = [iwdg])]
    fn idle(cx: idle::Context) -> ! {
        cx.local.iwdg.start(3000.millis());

        loop {
            cx.local.iwdg.feed();
            // cx.local.led.toggle();
            rtic::export::wfi();
        }
    }

    // #[task(binds=BusFault)]
    // fn bus_fault(_cx: bus_fault::Context) {
    //     panic!("BusFault");
    // }

    // #[task(binds=UsageFault)]
    // fn usage_fault(_cx: usage_fault::Context) {
    //     panic!("UsageFault");
    // }
}

// #[inline(never)]
// #[panic_handler]
// fn panic(text: &PanicInfo) -> ! {

//     let mut message = heapless::String::<256>::default();

//     if write!(message, "{text}").is_err() {
//         let _ = write!(message, "Could not format panic message");
//     }

//     info!("PANIC MESSAGE: {}", *message);

//     loop {
//         // add some side effect to prevent this from turning into a UDF instruction
//         // see rust-lang/rust#28728 for details
//         atomic::compiler_fence(Ordering::SeqCst);
//     }
// }
