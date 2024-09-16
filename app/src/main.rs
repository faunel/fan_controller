#![no_main]
#![no_std]
// #![allow(unused)]

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SPI3, SPI4])]
mod app {
    use async_button::prelude::*;
    use display_interface_spi::SPIInterface;
    use eeprom::eeprom::{Settings, EEPROM};
    use embassy_futures::select::{select3, Either3};
    use embedded_hal_bus::spi::ExclusiveDevice;
    use measurements::{
        control::Control,
        measure::{AdcMeasure, Data, ImpulsesComplete, ImpulsesRaw},
        ADC_BUFFER,
    };
    use mipidsi::{
        error::Error as DisplayError,
        models::ST7735s,
        options::{ColorOrder, Orientation, Rotation},
        Builder,
    };
    use monotonic::prelude::*;
    use ntc::Ntc;
    use stm32f4::stm32f401::{ADC1, DMA2, TIM4, TIM5, TIM9};
    use stm32f4xx_hal::{
        adc::{
            config::{AdcConfig, Clock, Continuous, Dma, Resolution, SampleTime, Scan, Sequence},
            Adc,
        },
        dma::{config::DmaConfig, PeripheralToMemory, Stream0, StreamsTuple, Transfer},
        gpio::{Edge, Output, Pin, Speed},
        i2c::{self, I2c},
        pac::{I2C2, TIM1, TIM11},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, NoMiso, Phase, Polarity, Spi},
        timer::{self, CounterHz, Event, Flag, Timer},
        watchdog::IndependentWatchdog,
    };
    use ui::setting::ItemSetting;
    use ui::Display;
    use ui::{
        main::MainScreen,
        screens::{start::StartScreen, Screen, Screens},
        setting::SettingScreen,
        Menu,
    };
    use {defmt_rtt as _, panic_probe as _};

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
        #[lock_free]
        impulses_complete: ImpulsesComplete,
        adc_buffer: Option<[u16; ADC_BUFFER]>,
    }

    #[local]
    struct Local {
        display: Display,
        led: Pin<'C', 13, Output>,
        fan1_exti: Pin<'B', 15>,
        fan2_exti: Pin<'B', 14>,
        fan3_exti: Pin<'B', 13>,
        fan4_exti: Pin<'B', 12>,
        tim_5: CounterHz<TIM5>,
        tim_9: CounterHz<TIM9>,
        tim_11: CounterHz<TIM11>,
        buffer: Option<&'static mut [u16; ADC_BUFFER]>,
        btn_minus_async: WaitPin<Pin<'A', 10>>,
        btn_ok_async: WaitPin<Pin<'A', 0>>,
        btn_plus_async: WaitPin<Pin<'A', 9>>,
        eeprom: EEPROM,
        adc: AdcMeasure,
        control: Control,
        iwdg: IndependentWatchdog,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let dp = cx.device;
        let cp = cx.core;

        // Clock configuration
        let clocks = dp.RCC.constrain().cfgr.use_hse(25.MHz()).sysclk(80.MHz()).freeze();

        // Monotonic timer
        Mono::start(cp.SYST, clocks.sysclk().to_Hz());

        // GPIO
        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();
        // let mut delay = cp.SYST.delay(&clocks);
        let mut delay = dp.TIM10.delay_us(&clocks);

        // cp.DCB.enable_trace();
        // cp.DWT.enable_cycle_counter();

        // let start = DWT::cycle_count();
        // info!("{}", DWT::cycle_count() - start);

        // SPI1 pin configuration
        let sck = gpioa.pa5.into_alternate().speed(Speed::VeryHigh);
        let mosi = gpioa.pa7.into_alternate().speed(Speed::VeryHigh);
        let miso = NoMiso::new();
        let rst = gpiob.pb0.into_push_pull_output().speed(Speed::Medium);
        let cs = gpiob.pb2.into_push_pull_output().speed(Speed::Medium);
        let dc = gpioa.pa15.into_push_pull_output().speed(Speed::VeryHigh);

        // Button pin configuration
        let btn_minus = gpioa.pa10.into_pull_up_input();
        let btn_ok: Pin<'A', 0> = gpioa.pa0.into_pull_up_input();
        let btn_plus = gpioa.pa9.into_pull_up_input();

        // Button async wrapper
        let btn_minus_async: WaitPin<Pin<'A', 10>> = WaitPin::new(btn_minus);
        let btn_ok_async = WaitPin::new(btn_ok);
        let btn_plus_async = WaitPin::new(btn_plus);

        // LED pin configuration
        let mut led = gpioc.pc13.into_push_pull_output();
        led.set_high();

        // EXTI pin configuration
        let mut fan1_exti = gpiob.pb15.into_pull_up_input();
        let mut fan2_exti = gpiob.pb14.into_pull_up_input();
        let mut fan3_exti = gpiob.pb13.into_pull_up_input();
        let mut fan4_exti = gpiob.pb12.into_pull_up_input();

        // ADC pin configuration
        let fan1_adc = gpioa.pa1.into_analog();
        let fan2_adc = gpioa.pa2.into_analog();
        let fan3_adc = gpioa.pa3.into_analog();
        let fan4_adc = gpioa.pa4.into_analog();

        // DMA stream
        let dma = StreamsTuple::new(dp.DMA2);

        // ADC config
        let adc_config = AdcConfig::default()
            .dma(Dma::Continuous)
            .scan(Scan::Enabled)
            .clock(Clock::Pclk2_div_8)
            .continuous(Continuous::Continuous)
            .resolution(Resolution::Twelve);

        // ADC channel configuration
        let mut adc = Adc::adc1(dp.ADC1, true, adc_config);
        adc.configure_channel(&fan1_adc, Sequence::One, SampleTime::Cycles_480);
        adc.configure_channel(&fan2_adc, Sequence::Two, SampleTime::Cycles_480);
        adc.configure_channel(&fan3_adc, Sequence::Three, SampleTime::Cycles_480);
        adc.configure_channel(&fan4_adc, Sequence::Four, SampleTime::Cycles_480);

        let adc_buffer = [0; ADC_BUFFER];
        let first_buffer = cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap();
        let adc_dma_buffer = Some(cortex_m::singleton!(: [u16; ADC_BUFFER] = [0; ADC_BUFFER]).unwrap());

        // DMA config
        let dma_config = DmaConfig::default().transfer_complete_interrupt(true).memory_increment(true).double_buffer(false);

        // DMA transfer
        let transfer = Transfer::init_peripheral_to_memory(dma.0, adc, first_buffer, None, dma_config);

        //---------------------------- Конфігурація апаратних переривань -----------------------

        let mut syscfg = dp.SYSCFG.constrain();
        let mut exti = dp.EXTI;

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

        // TIM4. PWM для вентиляторів
        // Рекомендована частота 25 кілогерц
        let ch_1: timer::ChannelBuilder<TIM4, 0> = timer::Channel1::new(gpiob.pb6);
        let ch_2: timer::ChannelBuilder<TIM4, 1> = timer::Channel2::new(gpiob.pb7);
        let ch_3: timer::ChannelBuilder<TIM4, 2> = timer::Channel3::new(gpiob.pb8);
        let ch_4: timer::ChannelBuilder<TIM4, 3> = timer::Channel4::new(gpiob.pb9);
        let timer = Timer::new(dp.TIM4, &clocks);
        let mut tim_4 = timer.pwm_hz((ch_1, ch_2, ch_3, ch_4), 25.kHz());
        tim_4.set_duty(timer::Channel::C1, 0);
        tim_4.set_duty(timer::Channel::C2, 0);
        tim_4.set_duty(timer::Channel::C3, 0);
        tim_4.set_duty(timer::Channel::C4, 0);
        tim_4.enable(timer::Channel::C1);
        tim_4.enable(timer::Channel::C2);
        tim_4.enable(timer::Channel::C3);
        tim_4.enable(timer::Channel::C4);
        let tim_4 = tim_4.split();

        // TIM5. Для відправки виміряних даних на дисплей (температура і оберти).
        let timer = Timer::new(dp.TIM5, &clocks);
        let mut tim_5 = timer.counter_hz();
        tim_5.start(25.Hz()).unwrap();
        tim_5.listen(Event::Update);

        // TIM9. Для вимірювання частоти вентиляторів.
        // Викликати раз на секунду
        let timer = Timer::new(dp.TIM9, &clocks);
        let mut tim_9 = timer.counter_hz();
        tim_9.start(1.Hz()).unwrap();
        tim_9.listen(Event::Update);

        // TIM11. Для запуску АЦП
        // Викликати раз в мілісекунду
        let timer: Timer<TIM11> = Timer::new(dp.TIM11, &clocks);
        let mut tim_11: CounterHz<TIM11> = timer.counter_hz();
        tim_11.start(100.Hz()).unwrap();
        tim_11.listen(Event::Update);

        // Для тесту RPM
        let timer = Timer::new(dp.TIM1, &clocks);
        let channels: timer::ChannelBuilder<TIM1, 0> = timer::Channel1::new(gpioa.pa8);
        let mut tim_1: timer::PwmHz<TIM1, timer::ChannelBuilder<TIM1, 0>> = timer.pwm_hz(channels, 10_000.Hz());
        tim_1.set_duty(timer::Channel::C1, tim_1.get_max_duty() / 2);
        tim_1.enable(timer::Channel::C1);

        //---------------------------------------------------

        // SPI1 mode configuration
        let mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };

        // SPI interface
        let spi = Spi::new(dp.SPI1, (sck, miso, mosi), mode, 10.MHz(), &clocks);
        let device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
        let interface = SPIInterface::new(device, dc);

        // Display configuration
        let display = Builder::new(ST7735s, interface)
            .orientation(Orientation {
                rotation: Rotation::Deg90,
                mirrored: false,
            })
            .color_order(ColorOrder::Rgb)
            .reset_pin(rst)
            .init(&mut delay)
            .unwrap();

        // Config I2C2
        let scl = gpiob.pb10;
        let sda = gpiob.pb3.into_floating_input();
        let i2c: I2c<I2C2> = I2c::new(dp.I2C2, (scl, sda), i2c::Mode::standard(100.kHz()), &clocks);

        let ntc = Ntc::default();

        // Tasks
        save::spawn().unwrap();
        button_task::spawn().unwrap();
        display_menu_task::spawn().unwrap();
        set_pwm_fan::spawn().unwrap();

        (
            Shared {
                data: Data::new(25, ntc),
                transfer,
                menu: Menu::Main,
                settings: Settings::new(),
                item_setting: ItemSetting::Item(1),
                is_clear: true,
                no_click_timer: None,
                impulses_raw: ImpulsesRaw::new(),
                impulses_complete: ImpulsesComplete::new(),
                adc_buffer: Some(adc_buffer),
            },
            Local {
                display,
                led,
                fan1_exti,
                fan2_exti,
                fan3_exti,
                fan4_exti,
                tim_5,
                tim_9,
                tim_11,
                buffer: adc_dma_buffer,
                btn_minus_async,
                btn_ok_async,
                btn_plus_async,
                eeprom: EEPROM::new(i2c),
                adc: AdcMeasure::new(),
                control: Control::new(tim_4),
                iwdg: IndependentWatchdog::new(dp.IWDG),
            },
        )
    }

    // Software task
    // Починаю робити відлік при відсутності натискань кнопок
    #[task(shared = [no_click_timer], priority = 2)]
    async fn no_click_timer(mut cx: no_click_timer::Context) {
        loop {
            cx.shared.no_click_timer.lock(|no_click_timer| {
                if let Some(t) = no_click_timer {
                    *t = t.saturating_sub(1);
                }
            });

            Mono::delay(1000.millis()).await;
        }
    }

    // Software task
    // Завантажує всі налаштування.
    // Зберігає налаштування при відсутності натискань кнопок за певний період при умові зміни будь якого параметру
    #[task(local = [eeprom], shared = [no_click_timer, settings, menu, is_clear, item_setting], priority = 2)]
    async fn save(mut cx: save::Context) {
        // let mut s = cx.shared.settings.lock(move |settings| settings.clone());
        // cx.local.eeprom.default_settings(&mut s).await;
        // cx.shared.settings.lock(|settings| {
        //     *settings = s;
        //     info!("default")
        // });

        let mut s = cx.shared.settings.lock(move |settings| settings.clone());
        cx.local.eeprom.load_settings(&mut s).await;
        cx.shared.settings.lock(|settings| {
            *settings = s;
        });

        loop {
            cx.shared.no_click_timer.lock(|no_click_timer| {
                if let Some(t) = no_click_timer {
                    *t = t.saturating_sub(1);
                }
            });

            let s = (&mut cx.shared.no_click_timer, &mut cx.shared.menu, &mut cx.shared.is_clear, &mut cx.shared.item_setting).lock(
                |no_click_timer, menu, is_clear, item_setting| {
                    if let Some(t) = no_click_timer {
                        if *t == 0 {
                            *no_click_timer = None;
                            *menu = Menu::Main;
                            *is_clear = true;
                            *item_setting = ItemSetting::Item(1);
                            return Some(cx.shared.settings.lock(|settings| settings.clone()));
                        }
                    }
                    None
                },
            );

            if let Some(mut s) = s {
                cx.local.eeprom.save_all(&mut s).await;
                // info!("SAVE");
            }

            Mono::delay(1000.millis()).await;
        }
    }

    // Software task
    // Для відображення даних на дисплеї
    #[task(local = [display, draw_static: bool = true], shared = [menu, data, settings, item_setting, is_clear], priority = 1)]
    async fn display_menu_task(cx: display_menu_task::Context) {
        // let display = unsafe { cx.shared.display.lock(|d| &mut *d.get()) };
        let display = cx.local.display;
        let mut shared = cx.shared;

        let mut screen: Screens<Display, DisplayError> = StartScreen::default().into();
        screen.draw_init(display).await;

        loop {
            (&mut shared.menu, &mut shared.is_clear).lock(|menu, is_clear| {
                match menu {
                    Menu::Main => {
                        *cx.local.draw_static = true;
                        // let data = shared.data.lock(|data| core::mem::replace(data, Data::new()));

                        shared.data.lock(|data| {
                            screen = Screens::Main(MainScreen::new(*data.get_temp(), *data.get_rpm(), *is_clear));
                        });
                    }
                    Menu::Fan(fan) => {
                        *cx.local.draw_static = true;
                        (&mut shared.settings, &mut shared.item_setting).lock(|settings, item_setting| {
                            screen = Screens::Setting(SettingScreen::new(
                                settings.fans[*fan - 1].clone(),
                                *fan,
                                item_setting.clone(),
                                *is_clear,
                            ));
                        });
                    }
                }
                *is_clear = false;
            });
            screen.draw_static(display);
            screen.draw_init(display).await;
            // Mono::delay(50.millis()).await;
        }
    }

    // Software task
    // Обробник натисання кнопок і зміна параметрів налаштувань
    #[task(local = [btn_minus_async, btn_plus_async, btn_ok_async], shared = [no_click_timer, settings, item_setting, menu, is_clear], priority = 2)]
    async fn button_task(mut cx: button_task::Context) {
        // Button configuration
        let button_config =
            ButtonConfig::new(MyDuration::millis(20), MyDuration::millis(1), MyDuration::millis(500), ButtonMode::PullUp, 80);

        let mut btn_minus = Button::new(cx.local.btn_minus_async, button_config);
        let mut btn_ok = Button::new(cx.local.btn_ok_async, button_config);
        let mut btn_plus = Button::new(cx.local.btn_plus_async, button_config);

        loop {
            let select = select3(btn_minus.update(), btn_ok.update(), btn_plus.update()).await;

            // let is_pressed_minus_plus = btn_minus.is_pin_pressed() && btn_plus.is_pin_pressed();
            // if is_pressed_minus_plus {
            //     Mono::delay(500.millis()).await;
            //     if is_pressed_minus_plus {
            //         cx.shared.menu.lock(|menu| {
            //             *menu = Menu::Settings;
            //         });
            //     }
            // }

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
                        },
                        ButtonEvent::LongPress => {}
                        ButtonEvent::LongPressDuration(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.decrement_logic(fan, item),
                            },
                            Menu::Main => {}
                        },
                    },
                    Either3::Second(ok) => match ok {
                        ButtonEvent::ShortPress(_) => {
                            if let Menu::Fan(_) = menu {
                                match item_setting {
                                    ItemSetting::Item(mut item) => {
                                        item += 1;
                                        if item > 8 {
                                            item = 1;
                                        }
                                        *item_setting = ItemSetting::Item(item);
                                    }
                                }
                            }
                        }
                        ButtonEvent::LongPress => {
                            *is_clear = true;

                            match menu {
                                Menu::Main => *menu = Menu::Fan(1),
                                Menu::Fan(mut fan) => {
                                    *item_setting = ItemSetting::Item(1);
                                    if fan < 4 {
                                        fan += 1;
                                        *menu = Menu::Fan(fan);
                                    } else {
                                        *menu = Menu::Main;
                                    }
                                }
                            }
                        }
                        ButtonEvent::LongPressDuration(_) => {}
                    },
                    Either3::Third(plus) => match plus {
                        ButtonEvent::ShortPress(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.increment_logic(fan, item),
                            },
                            Menu::Main => {}
                        },
                        ButtonEvent::LongPress => {}
                        ButtonEvent::LongPressDuration(_) => match menu {
                            Menu::Fan(fan) => match item_setting {
                                ItemSetting::Item(item) => settings.increment_logic(fan, item),
                            },
                            Menu::Main => {}
                        },
                    },
                },
            );
        }
    }

    // Sowtware task
    // Управління логікою на основі виміряних даних
    #[task(local = [control], shared = [data, settings], priority = 2)]
    async fn set_pwm_fan(mut cx: set_pwm_fan::Context) {
        loop {
            (&mut cx.shared.data, &mut cx.shared.settings).lock(|data, settings| {
                // let temp = data.get_temp();
                cx.local.control.run(settings, data);
            });
            Mono::delay(200.millis()).await;
        }
    }

    // Hardware task
    // TIM5. Для відправки виміряних даних на дисплей (температура і оберти).
    #[task(binds = TIM5, local = [tim_5, adc], shared = [adc_buffer, impulses_complete, data], priority = 3)]
    fn tim_5(mut cx: tim_5::Context) {
        if cx.local.tim_5.flags().contains(Flag::Update) {
            cx.local.tim_5.clear_flags(Flag::Update);
        }

        let adc_buffer = cx.shared.adc_buffer.lock(|adc_buffer| adc_buffer.take());
        if let Some(buffer) = &adc_buffer {
            let adc_values: &[u16; 4] = cx.local.adc.split_channels(buffer);
            cx.shared.data.lock(|data| data.set_temp(adc_values));
        }
        cx.shared.data.lock(|data| data.set_rpm(cx.shared.impulses_complete));
    }

    // Hardware task
    // TIM9. Для вимірювання частоти вентиляторів.
    #[task(binds = TIM1_BRK_TIM9, local = [tim_9], shared = [impulses_raw, impulses_complete], priority = 3)]
    fn tim_9(mut cx: tim_9::Context) {
        if cx.local.tim_9.flags().contains(Flag::Update) {
            cx.local.tim_9.clear_flags(Flag::Update);
        }

        cx.shared.impulses_raw.lock(|impulses_raw| {
            for (ind, fan) in impulses_raw.iter_mut().enumerate() {
                cx.shared.impulses_complete.set(ind, fan);
                *fan = 0;
            }
        });
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
    // Частоти вентилятора 1. Викликається при надходженні сигналу на pin
    #[task(binds = EXTI15_10, local = [fan1_exti, fan2_exti, fan3_exti, fan4_exti], shared = [impulses_raw], priority = 4)]
    fn exti_15(mut cx: exti_15::Context) {
        cx.shared.impulses_raw.lock(|impulses_raw| {
            if cx.local.fan1_exti.check_interrupt() {
                impulses_raw.add_raw(0);
                cx.local.fan1_exti.clear_interrupt_pending_bit();
            }
            if cx.local.fan2_exti.check_interrupt() {
                impulses_raw.add_raw(1);
                cx.local.fan2_exti.clear_interrupt_pending_bit();
            }
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
    #[task(binds = DMA2_STREAM0, shared = [transfer, adc_buffer], local = [buffer, a: u32 = 0], priority = 5)]
    fn dma(mut cx: dma::Context) {
        let buffer = cx.shared.transfer.lock(|transfer| {
            let (buffer, _) = transfer.next_transfer(cx.local.buffer.take().unwrap()).unwrap();

            buffer
        });

        if *cx.local.a % 10 == 0 {
            // info!("adc_buffer: {}", buffer[0..16]);
        }

        cx.shared.adc_buffer.lock(|adc_buffer| {
            *adc_buffer = Some(*buffer);
        });

        *cx.local.buffer = Some(buffer);

        *cx.local.a += 1;
    }

    #[idle(local = [led, iwdg])]
    fn idle(cx: idle::Context) -> ! {
        cx.local.iwdg.start(3000.millis());

        loop {
            cx.local.iwdg.feed();
            // cx.local.led.toggle();
            rtic::export::wfi();
        }
    }
}
