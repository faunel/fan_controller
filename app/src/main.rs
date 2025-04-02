#![no_main]
#![no_std]
// #![allow(unused)]

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI2, SPI3, SPI4])]
mod app {
    use async_button::prelude::*;
    // use cortex_m::asm;
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
        measure::{AdcMeasure, Data, MeasureConfig, RpmData, RpmState},
        ADC_BUFFER,
    };
    use mipidsi::interface::SpiInterface;
    use mipidsi::{models::ST7789, options::ColorInversion};
    use mipidsi::{
        // interface::SpiError as DisplayError,
        // s::Error as DisplayError,
        options::{ColorOrder, Orientation, Rotation},
        Builder,
    };
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
        gpio::{Pin, Speed},
        hal::pwm::SetDutyCycle,
        i2c::{self, I2c},
        pac::{ADC1, DMA2, TIM1, TIM11, TIM3, TIM5},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, NoMiso, Phase, Polarity, Spi},
        timer::{self, Event, Flag, PwmChannel, Timer},
        watchdog::IndependentWatchdog,
    };
    use stm32f4xx_hal::{
        gpio::{Analog, Input, Output, PinState, Pull},
        timer::{CaptureChannel, CaptureHzManager, CounterHz},
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

    // use stm32ral::{gpio, rcc, spi::SPI1};
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
        adc_buffer: Option<[u16; ADC_BUFFER]>,
        control: Control,
        rpm_data: [RpmData; 4],
        // pwm_input_one: PwmInputOne,
        // pwm_input_two: PwmInputTwo,
        // duty_cycle_fan1: u8,
        // duty_cycle_fan2: u8,
    }

    #[local]
    struct Local {
        rpm_tim: CaptureHzManager<TIM5>,
        rpm_channels: (CaptureChannel<TIM5, 3>, CaptureChannel<TIM5, 1>, CaptureChannel<TIM5, 2>, CaptureChannel<TIM5, 0>),
        tim_11: CounterHz<TIM11>,
        buffer: Option<&'static mut [u16; ADC_BUFFER]>,
        btn_minus_async: WaitPin<Pin<'B', 15>>,
        btn_ok_async: WaitPin<Pin<'A', 9>>,
        btn_plus_async: WaitPin<Pin<'A', 10>>,
        adc: AdcMeasure,
        eeprom: EEPROM,
        iwdg: IndependentWatchdog,
        bl_tim: timer::PwmChannel<TIM1, 0>,
        rpm_state: [RpmState; 4],
        dummy_fans: (PwmChannel<TIM3, 0>, PwmChannel<TIM3, 1>),
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
            unsafe {
                HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE)
            }
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

        let mut delay = dp.TIM10.delay_us(&clocks);

        // cp.DCB.enable_trace();
        // cp.DWT.enable_cycle_counter();

        // let start = DWT::cycle_count();
        // info!("{}", DWT::cycle_count() - start);

        //---------------------------- Start GPIO config -----------------------
        // SPI1 pin config
        let sck_pin = gpioa.pa5.into_alternate().speed(Speed::VeryHigh);
        let mosi_pin = gpioa.pa7.into_alternate().speed(Speed::VeryHigh);
        let miso_pin = NoMiso::new();
        let rst_pin: Pin<'B', 14, Output> = Output::new(gpiob.pb14, PinState::Low).speed(Speed::Medium);
        let cs_pin: Pin<'B', 12, Output> = Output::new(gpiob.pb12, PinState::Low).speed(Speed::Medium);
        let dc_pin: Pin<'B', 13, Output> = Output::new(gpiob.pb13, PinState::Low).speed(Speed::VeryHigh);

        // I2C2 pin config
        let scl_pin = Input::new(gpiob.pb10, Pull::Up);
        let sda_pin = Input::new(gpiob.pb3, Pull::Up);

        // Button pin config
        let btn_minus_pin = Input::new(gpiob.pb15, Pull::Up);
        let btn_ok_pin: Pin<'A', 9> = Input::new(gpioa.pa9, Pull::Up);
        let btn_plus_pin = Input::new(gpioa.pa10, Pull::Up);

        // Button async wrapper
        let btn_minus_async = WaitPin::new(btn_minus_pin);
        let mut btn_ok_async: WaitPin<Pin<'A', 9>> = WaitPin::new(btn_ok_pin);
        let btn_plus_async = WaitPin::new(btn_plus_pin);

        // Buzzer pin
        let _buzzer_pin: Pin<'A', 15, Output> = Output::new(gpioa.pa15, PinState::Low);

        // Backlight pin
        let backlight_pin = gpioa.pa8;

        // Fan PWM output pin
        let fan1_pwm_pin = gpiob.pb9;
        let fan2_pwm_pin = gpiob.pb6;
        let fan3_pwm_pin = gpiob.pb8;
        let fan4_pwm_pin = gpiob.pb7;

        // Fan RPM input_capture pin
        let fan1_rpm_pin = gpioa.pa3;
        let fan2_rpm_pin = gpioa.pa1;
        let fan3_rpm_pin = gpioa.pa2;
        let fan4_rpm_pin = gpioa.pa0;

        // Fan emulator PWM out pin
        let fan_emulator_pwm1_pin: Pin<'B', 4, Output> = Output::new(gpiob.pb4, PinState::Low);
        let fan_emulator_pwm2_pin: Pin<'B', 5, Output> = Output::new(gpiob.pb5, PinState::Low);

        // ADC pin config
        let ntc1_pin = Analog::new(gpioa.pa4);
        let ntc2_pin = Analog::new(gpioa.pa6);
        let ntc3_pin = Analog::new(gpiob.pb0);
        let ntc4_pin = Analog::new(gpiob.pb1);
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
        adc.configure_channel(&ntc1_pin, Sequence::One, SampleTime::Cycles_480);
        adc.configure_channel(&ntc2_pin, Sequence::Two, SampleTime::Cycles_480);
        adc.configure_channel(&ntc3_pin, Sequence::Three, SampleTime::Cycles_480);
        adc.configure_channel(&ntc4_pin, Sequence::Four, SampleTime::Cycles_480);

        // ADC buffers
        let adc_buffer = [0; ADC_BUFFER];
        let first_buffer = cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap();
        let adc_dma_buffer = Some(cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap());

        // DMA config
        let dma_config = DmaConfig::default().transfer_complete_interrupt(true).memory_increment(true).double_buffer(false);

        // DMA transfer
        let transfer = Transfer::init_peripheral_to_memory(dma.0, adc, first_buffer, None, dma_config);
        //---------------------------- Finish ADC config -----------------------

        //---------------------------- Start hardware interrupts -----------------------

        // TIM1. For backlight. PWM output config
        let timer = Timer::new(dp.TIM1, &clocks);
        let (_, (ch1, ..)) = timer.pwm_hz(20.kHz());
        let mut bl_tim: timer::PwmChannel<TIM1, 0> = ch1.with(backlight_pin);
        bl_tim.set_duty(50);
        bl_tim.enable();

        // TIM3. For dummy fan
        let timer = Timer::new(dp.TIM3, &clocks);
        let (_, (ch1, ch2, ..)) = timer.pwm_hz(300.Hz());
        let mut dummy_fan_1 = ch1.with(fan_emulator_pwm1_pin);
        let mut dummy_fan_2 = ch2.with(fan_emulator_pwm2_pin);
        dummy_fan_1.set_duty_cycle_percent(50).unwrap();
        dummy_fan_2.set_duty_cycle_percent(50).unwrap();
        dummy_fan_1.enable();
        dummy_fan_2.enable();

        let dummy_fans = (dummy_fan_1, dummy_fan_2);

        // TIM4. PWM для вентиляторів. PWM output config
        let timer = Timer::new(dp.TIM4, &clocks);
        let (_, (ch1, ch2, ch3, ch4, ..)) = timer.pwm_hz(25.kHz());

        let mut fan2_pwm = ch1.with(fan2_pwm_pin);
        let mut fan4_pwm = ch2.with(fan4_pwm_pin);
        let mut fan3_pwm = ch3.with(fan3_pwm_pin);
        let mut fan1_pwm = ch4.with(fan1_pwm_pin);

        // fan1_pwm.set_polarity(timer::Polarity::ActiveLow);
        // fan2_pwm.set_polarity(timer::Polarity::ActiveLow);
        // fan3_pwm.set_polarity(timer::Polarity::ActiveLow);
        // fan4_pwm.set_polarity(timer::Polarity::ActiveLow);

        fan1_pwm.set_duty(0);
        fan2_pwm.set_duty(0);
        fan3_pwm.set_duty(0);
        fan1_pwm.set_duty(0);

        fan1_pwm.enable();
        fan2_pwm.enable();
        fan3_pwm.enable();
        fan4_pwm.enable();

        let pwm_tim = (fan1_pwm, fan2_pwm, fan3_pwm, fan4_pwm);

        // TIM5 for RPM measure
        let (mut rpm_tim, (tim5_ch1, tim5_ch2, tim5_ch3, tim5_ch4)) = Timer::new(dp.TIM5, &clocks).capture_hz(1.MHz());
        let mut fan4_rpm = tim5_ch1.with(fan4_rpm_pin);
        let mut fan2_rpm = tim5_ch2.with(fan2_rpm_pin);
        let mut fan3_rpm = tim5_ch3.with(fan3_rpm_pin);
        let mut fan1_rpm = tim5_ch4.with(fan1_rpm_pin);

        // Полярність імпульсів
        fan1_rpm.set_polarity(timer::Polarity::ActiveHigh);
        fan2_rpm.set_polarity(timer::Polarity::ActiveHigh);
        fan3_rpm.set_polarity(timer::Polarity::ActiveHigh);
        fan4_rpm.set_polarity(timer::Polarity::ActiveHigh);

        // Дільник імпульсів
        // fan1_rpm.set_prescaler(timer::CapturePrescaler::Eight);
        // fan2_rpm.set_prescaler(timer::CapturePrescaler::Eight);
        // fan3_rpm.set_prescaler(timer::CapturePrescaler::Eight);
        // fan4_rpm.set_prescaler(timer::CapturePrescaler::Eight);

        // Фільтр імпульсів
        fan1_rpm.set_filter(timer::CaptureFilter::FdtsDiv32N8);
        fan2_rpm.set_filter(timer::CaptureFilter::FdtsDiv32N8);
        fan3_rpm.set_filter(timer::CaptureFilter::FdtsDiv32N8);
        fan4_rpm.set_filter(timer::CaptureFilter::FdtsDiv32N8);

        // Вмикаємо обробник переривання для input capture
        rpm_tim.listen(Event::C1);
        rpm_tim.listen(Event::C2);
        rpm_tim.listen(Event::C3);
        rpm_tim.listen(Event::C4);

        // Вмикаємо канали таймера
        fan1_rpm.enable();
        fan2_rpm.enable();
        fan3_rpm.enable();
        fan4_rpm.enable();

        let rpm_channels = (fan1_rpm, fan2_rpm, fan3_rpm, fan4_rpm);

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

        let spi = Spi::new(dp.SPI1, (sck_pin, miso_pin, mosi_pin), mode, 48.MHz(), &clocks);
        let device = ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
        let interface = SpiInterface::new(device, dc_pin, spi_buffer);

        // Display config
        let display: Display = Builder::new(ST7789, interface)
            .orientation(Orientation {
                rotation: Rotation::Deg90,
                mirrored: false,
            })
            .display_offset(0, 20)
            .color_order(ColorOrder::Rgb)
            .invert_colors(ColorInversion::Inverted)
            .reset_pin(rst_pin)
            .display_size(240, 280)
            .init(&mut delay)
            .unwrap();

        let ntc = Ntc::default();
        let measure_config = MeasureConfig {
            temp_ema_window: 20,
            rpm_ema_window: 20,
        };
        let data = Data::new(ntc, measure_config);

        // Tasks
        eeprom_task::spawn(&mut btn_ok_async, display).ok();
        data_task::spawn().unwrap();
        clear_rpm_task::spawn().unwrap();

        // Ці задачі повинні бути створені після задачі "eeprom_task"
        // Це пояснюється тим, що для цих задач структура settings повинна бути з даними, отриманими з EEPROM

        // display_menu_task::spawn(display).ok();
        // button_task::spawn().unwrap();
        // backlight_task::spawn().unwrap();
        // pwm_fan_task::spawn().unwrap();

        (
            Shared {
                data,
                transfer,
                menu: Menu::Main,
                settings: Settings::new(),
                item_setting: ItemSetting::Item(1),
                is_clear: true,
                no_click_timer: None,
                adc_buffer: Some(adc_buffer),
                control: Control::new(pwm_tim),
                rpm_data: RpmData::new(),
                // pwm_input_one,
                // pwm_input_two,
                // duty_cycle_fan1: 200,
                // duty_cycle_fan2: 200,
            },
            Local {
                rpm_tim,
                rpm_channels,
                tim_11,
                buffer: adc_dma_buffer,
                btn_minus_async,
                btn_ok_async,
                btn_plus_async,
                adc: AdcMeasure::new(),
                eeprom: EEPROM::new(I2c::new(dp.I2C2, (scl_pin, sda_pin), i2c::Mode::standard(100.kHz()), &clocks)),
                iwdg: IndependentWatchdog::new(dp.IWDG),
                bl_tim,
                rpm_state: RpmState::new(),
                dummy_fans,
            },
        )
    }

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

    #[task(local = [dummy_fans], shared = [rpm_data, control], priority = 2)]
    async fn clear_rpm_task(mut cx: clear_rpm_task::Context) {
        loop {
            cx.shared.rpm_data.lock(|rpm_data| {
                rpm_data.iter_mut().for_each(|r| r.clear_active_fans());
            });
            Mono::delay(2000.millis()).await;

            cx.shared.rpm_data.lock(|rpm_data| {
                if !rpm_data[0].is_active_fan() {
                    rpm_data[0].clear_rpm();
                    cx.shared.control.lock(|control| {
                        let duty = control.get_duty_cycle_percent_ch1();
                        if duty > 10 {
                            cx.local.dummy_fans.0.disable();
                        }
                    });
                } else {
                    cx.local.dummy_fans.0.enable();
                }
                if !rpm_data[1].is_active_fan() {
                    rpm_data[1].clear_rpm();
                    cx.shared.control.lock(|control| {
                        let duty = control.get_duty_cycle_percent_ch2();
                        if duty > 10 {
                            cx.local.dummy_fans.1.disable();
                        }
                    });
                } else {
                    cx.local.dummy_fans.1.enable();
                }
                if !rpm_data[2].is_active_fan() {
                    rpm_data[2].clear_rpm();
                }
                if !rpm_data[3].is_active_fan() {
                    rpm_data[3].clear_rpm();
                }
            });
        }
    }

    // Software task
    // Завантажує всі налаштування.
    // Зберігає налаштування при відсутності натискань кнопок за певний період при умові зміни будь якого параметру
    #[task(local = [eeprom], shared = [no_click_timer, settings, menu, is_clear, item_setting], priority = 2)]
    async fn eeprom_task(mut cx: eeprom_task::Context, btn_ok_async: &mut WaitPin<Pin<'A', 9>>, display: Display) {
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

        button_task::spawn().unwrap();
        backlight_task::spawn().unwrap();
        pwm_fan_task::spawn().unwrap();
        display_menu_task::spawn(display).ok();

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
    async fn display_menu_task(cx: display_menu_task::Context, mut display: Display) {
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

            Mono::delay(10.millis()).await;
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
                                } else if settings.ntc_no[item - 2].data > 1 {
                                    settings.ntc_no[item - 2].data -= 1;
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
                                } else if settings.ntc_no[item - 2].data < 4 {
                                    settings.ntc_no[item - 2].data += 1;
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
    #[task(local = [bl_tim], shared = [settings], priority = 2)]
    async fn backlight_task(mut cx: backlight_task::Context) {
        loop {
            Mono::delay(100.millis()).await;
            cx.shared.settings.lock(|settings| {
                let mut percent = settings.backlight.data as u8;
                percent = if percent > 0 { percent * 10 } else { 1 };
                cx.local.bl_tim.set_duty_cycle_percent(percent).unwrap();
            });
        }
    }

    // Sowtware task
    // Управління вентиляторами на основі виміряних даних
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

    // Sowtware task
    // Запис виміряних даних в структуру
    #[task(local = [adc], shared = [adc_buffer, data, rpm_data], priority = 3)]
    async fn data_task(mut cx: data_task::Context) {
        loop {
            let adc_buffer = cx.shared.adc_buffer.lock(|adc_buffer| adc_buffer.take());
            if let Some(buffer) = &adc_buffer {
                let adc_values: &[u16; 4] = cx.local.adc.split_channels(buffer);
                cx.shared.data.lock(|data| data.set_temp(adc_values));
            }

            (&mut cx.shared.data, &mut cx.shared.rpm_data).lock(|data, rpm_data| {
                let mut rpm: [u16; 4] = [0; 4];
                for (i, data) in rpm_data.iter().enumerate() {
                    rpm[i] = data.get_rpm();
                }
                data.set_rpm(&rpm)
            });

            Mono::delay(20.millis()).await;
        }
    }

    // Hardware task
    // TIM5. Для вимірювання частоти вентиляторів.
    #[task(binds = TIM5, local = [rpm_tim, rpm_channels, rpm_state], shared = [data, rpm_data], priority = 4)]
    fn tim5_interrupt(mut cx: tim5_interrupt::Context) {
        let timer_clock = cx.local.rpm_tim.get_timer_clock();
        let max_auto_reload = cx.local.rpm_tim.get_max_auto_reload();

        let (ch1, ch2, ch3, ch4) = cx.local.rpm_channels;


        if cx.local.rpm_tim.flags().contains(Flag::C1) {
            let fan_number = 3;
            let current_capture = ch4.get_capture();
            let prev_capture = cx.local.rpm_state[fan_number].get_prev_capture();

            let delta = if current_capture >= prev_capture {
                current_capture - prev_capture
            } else {
                (max_auto_reload - prev_capture) + current_capture
            };

            let freq = timer_clock as f32 / delta as f32;
            if let Some(rpm) = cx.local.rpm_state[fan_number].calculate_rpm(freq) {
                info!("rpm: {}", rpm);
                cx.shared.rpm_data.lock(|rpm_data| {
                    rpm_data[fan_number].set_rpm(rpm);
                    rpm_data[fan_number].set_active_fan();
                });
            }
            cx.local.rpm_state[fan_number].set_prev_capture(current_capture);
            cx.local.rpm_tim.clear_flags(Flag::C1);
        }

        if cx.local.rpm_tim.flags().contains(Flag::C2) {
            let fan_number = 1;
            let current_capture = ch2.get_capture();
            let prev_capture = cx.local.rpm_state[fan_number].get_prev_capture();

            let delta = if current_capture >= prev_capture {
                current_capture - prev_capture
            } else {
                (max_auto_reload - prev_capture) + current_capture
            };

            let freq = timer_clock as f32 / delta as f32;
            if let Some(rpm) = cx.local.rpm_state[fan_number].calculate_rpm(freq) {
                info!("rpm: {}", rpm);
                cx.shared.rpm_data.lock(|rpm_data| {
                    rpm_data[fan_number].set_rpm(rpm);
                    rpm_data[fan_number].set_active_fan();
                });
            }
            cx.local.rpm_state[fan_number].set_prev_capture(current_capture);
            cx.local.rpm_tim.clear_flags(Flag::C2);
        }

        if cx.local.rpm_tim.flags().contains(Flag::C3) {
            let fan_number = 2;
            let current_capture = ch3.get_capture();
            let prev_capture = cx.local.rpm_state[fan_number].get_prev_capture();

            let delta = if current_capture >= prev_capture {
                current_capture - prev_capture
            } else {
                (max_auto_reload - prev_capture) + current_capture
            };

            let freq = timer_clock as f32 / delta as f32;
            if let Some(rpm) = cx.local.rpm_state[fan_number].calculate_rpm(freq) {
                info!("rpm: {}", rpm);
                cx.shared.rpm_data.lock(|rpm_data| {
                    rpm_data[fan_number].set_rpm(rpm);
                    rpm_data[fan_number].set_active_fan();
                });
            }
            cx.local.rpm_state[fan_number].set_prev_capture(current_capture);
            cx.local.rpm_tim.clear_flags(Flag::C3);
        }

        if cx.local.rpm_tim.flags().contains(Flag::C4) {
            let fan_number = 0;
            let current_capture = ch1.get_capture();
            let prev_capture = cx.local.rpm_state[fan_number].get_prev_capture();

            let delta = if current_capture >= prev_capture {
                current_capture - prev_capture
            } else {
                (max_auto_reload - prev_capture) + current_capture
            };

            let freq = timer_clock as f32 / delta as f32;
            if let Some(rpm) = cx.local.rpm_state[fan_number].calculate_rpm(freq) {
                info!("rpm: {}", rpm);
                cx.shared.rpm_data.lock(|rpm_data| {
                    rpm_data[fan_number].set_rpm(rpm);
                    rpm_data[fan_number].set_active_fan();
                });
            }
            cx.local.rpm_state[fan_number].set_prev_capture(current_capture);
            cx.local.rpm_tim.clear_flags(Flag::C4);
        }
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
        cx.local.iwdg.start(1000.millis());

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
