#![no_main]
#![no_std]
#![allow(unused)]

mod init;
mod instant;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [SPI3, SPI4])]
mod app {
    use crate::{
        init::{Init},
        instant::{GLOBAL_TIMER_COUNTER, TIMER_PERIOD},
    };
    use async_button::prelude::*;
    // use core::cell::UnsafeCell;
    use defmt::info;
    use display_interface_spi::SPIInterface;
    use eeprom::eeprom::{SettingFan, Settings, EEPROM};
    use embedded_graphics::draw_target::DrawTarget;
    use embedded_hal_bus::spi::ExclusiveDevice;
    use heapless::Vec;
    use measurements::data::Data;
    use mipidsi::{
        models::ST7735s,
        options::{ColorOrder, Orientation, Rotation},
        Builder,
        error::Error as DisplayError
    };
    use monotonic::prelude::*;
    use stm32f4::stm32f401::{ADC1, DMA2};
    use stm32f4xx_hal::{
        adc::{
            config::{AdcConfig, Clock, Continuous, Dma, Resolution, SampleTime, Scan, Sequence},
            Adc,
        },
        dma::{config::DmaConfig, PeripheralToMemory, Stream0, StreamsTuple, Transfer},
        gpio::{Edge, Output, Pin, Speed},
        i2c::{self, I2c},
        pac::{I2C2, TIM10, TIM11, SPI1},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, NoMiso, Phase, Polarity, Spi},
        timer::{self, CounterHz, Event, Flag, Timer},
    };
    use ui::{main::MainScreen, menu::{AllButton, ButtonState, Menu}, screens::{start::StartScreen, Screen, Screens}, setting::SettingScreen};
    use ui::setting::ItemSetting;

    use embassy_futures::select::{select3, Either3};
    use ui::{Display, BACKGROUND_COLOR};
    use {defmt_rtt as _, panic_probe as _};
    use display_interface::WriteOnlyDataCommand;

    type DMATransfer =
        Transfer<Stream0<DMA2>, 0, Adc<ADC1>, PeripheralToMemory, &'static mut [u16; 2]>;

    #[shared]
    struct Shared {
        // display: UnsafeCell<Display>,
        data: Vec<Data, 4>,
        frequency: u16,
        #[lock_free]
        interrupt_count: u16,
        transfer: DMATransfer,
        menu: Menu,
        settings: Settings,
        item_setting: ItemSetting,
        is_clear: bool,
        eeprom: EEPROM,
        buttons: AllButton
    }

    #[local]
    struct Local {
        display: Display,
        led: Pin<'C', 13, Output>,
        exti_pin: Pin<'B', 15>,
        timer_11: CounterHz<TIM11>,
        buffer: Option<&'static mut [u16; 2]>,
        btn_minus_async: WaitPin<Pin<'A', 10>>,
        btn_ok_async: WaitPin<Pin<'A', 0>>,
        btn_plus_async: WaitPin<Pin<'A', 9>>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let dp = cx.device;
        let cp = cx.core;

            let init = Init::new(dp, cp);

            let mut syscfg = init.syscfg();

        //     // syscfg
            // let mut syscfg = dp.SYSCFG.constrain();

        //     // Clock configuration
        //     let clocks = init.clocks();

        //     // GPIO
        //     let (gpioa, gpiob, gpioc) = init.gpio();

        //     // let mut delay = cp.SYST.delay(&clocks);
        //     let mut delay = init.delay(&clocks);

        //     // Monotonic timer
        //    init.mono_start(&clocks);

        // let dp = cx.device;
        // let cp = cx.core;

        // Init;

        // syscfg
        // let mut syscfg = dp.SYSCFG.constrain();

        let clocks = init.clocks();

        let gpio = init.gpio();

      
 
        let mut delay: timer::Delay<TIM10, 1000000> = init.delay(&clocks);
        init.mono_start(&clocks);

        // Monotonic timer
        //Mono::start(cp.SYST, clocks.sysclk().to_Hz());

        //let mut syscfg = dp.SYSCFG.constrain();

        // eeprom.read(&mut settings.fans[0].thresholds[0].temp);
        // eeprom.read(&mut settings.fans[0].thresholds[0].pwm);

        // eeprom.read(&mut settings.fans[0].thresholds[1].temp);
        // eeprom.read(&mut settings.fans[0].thresholds[1].pwm);

        // eeprom.read(&mut settings.fans[0].thresholds[2].temp);
        // eeprom.read(&mut settings.fans[0].thresholds[2].pwm);

        // eeprom.read(&mut settings.fans[0].thresholds[3].temp);
        // eeprom.read(&mut settings.fans[0].thresholds[3].pwm);

        // info!("data: {}", settings.fan4.temp.0);
        // info!("data: {}", settings.fan4.get_temp());
        // info!("data: {}", settings.get(settings.fan4.temp));

        // eeprom.save(&mut settings.fan1.temp, 5566);
        // info!("data: {}", settings.get(settings.fan1.temp));

        // eeprom.save(&mut settings.fans[0].thresholds[0].temp, 30);
        // eeprom.save(&mut settings.fans[0].thresholds[0].pwm, 20);

        // eeprom.save(&mut settings.fans[0].thresholds[1].temp, 40);
        // eeprom.save(&mut settings.fans[0].thresholds[1].pwm, 35);

        // eeprom.save(&mut settings.fans[0].thresholds[2].temp, 50);
        // eeprom.save(&mut settings.fans[0].thresholds[2].pwm, 60);

        // eeprom.save(&mut settings.fans[0].thresholds[3].temp, 70);
        // eeprom.save(&mut settings.fans[0].thresholds[3].pwm, 100);

        // let value = eeprom.read(settings.fan1.pwm);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan2.temp);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan2.pwm);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan3.temp);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan3.pwm);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan4.temp);
        // info!("data: {}", value);

        // let value = eeprom.read(settings.fan4.pwm);
        // info!("data: {}", value);

        // SPI1 pin configuration
        let spi1_pin = init.spi1_pin();


        // Button pin configuration
        let button = init.button();
       

        // LED pin configuration
        let mut led = init.gpioc.pc13.into_push_pull_output();
        led.set_high();

        // EXTI pin configuration
        let exti_pin = init.exti_pin();
      

        let adc_config = init.adc_config();
        let adc_channel_config = init.adc_channel_config(adc_config);
        let dma_config = init.dma_config();

        // These buffers need to be 'static to use safely with the DMA - we can't allow them to be dropped while the DMA is accessing them.
        // The easiest way to satisfy that is to make them static, and the safest way to do that is with `cortex_m::singleton!`
        let first_buffer: &mut [u16; 2] = cortex_m::singleton!(: [u16; 2] = [0; 2]).unwrap();
        let adc_dma_buffer = Some(cortex_m::singleton!(: [u16; 2] = [0; 2]).unwrap());
        let transfer = init.transfer(adc_channel_config, dma_config, first_buffer);
    

        transfer.start(|adc| {
            adc.start_conversion();
            // info!("conversion");
        });


        let timer_11 = init.timer_11_config(&clocks);


        // Test timer
        let timer_1 = init.timer_1_config(&clocks);

        let spi_pin = init.spi1_pin();
        let interface = init.spi_interface(spi_pin, &clocks);

        let display = init.display(interface, spi_pin.rst, &mut delay);


        // display.clear(BACKGROUND_COLOR).unwrap();

        // delay.delay_ms(1000);

        // Configure I2C2
        let i2c = init.i2c(&clocks);


        let mut settings = Settings::new();

        let mut eeprom = EEPROM::new(i2c, delay);
        eeprom.load_settings(&mut settings);

        // info!("data: {}", settings.fans[0].thresholds[0].get_temp());
        // info!("data: {}", settings.fans[0].thresholds[0].get_pwm());

        // info!("data: {}", settings.fans[0].thresholds[1].get_temp());
        // info!("data: {}", settings.fans[0].thresholds[1].get_pwm());

        // info!("data: {}", settings.fans[0].thresholds[2].get_temp());
        // info!("data: {}", settings.fans[0].thresholds[2].get_pwm());

        // info!("data: {}", settings.fans[0].thresholds[3].get_temp());
        // info!("data: {}", settings.fans[0].thresholds[3].get_pwm());

        // Fan data
        let data: Vec<Data, 4> = Data::new();

        // Tasks
        display_menu_task::spawn().unwrap();
        button_task::spawn().unwrap();
        test_task_rpm::spawn().unwrap();
        adc_start_task::spawn().unwrap();
        button_menu_task::spawn().unwrap();

        // button_ok_task::spawn().unwrap();
        // button_plus_task::spawn().unwrap();
        // time_task::spawn().unwrap();
        // test_task_temp::spawn().unwrap();


        (
            Shared {
                // display: UnsafeCell::new(display),
                data,
                frequency: 0,
                interrupt_count: 0,
                transfer,
                // button: Button::No,
                menu: Menu::Main,
                settings,
                item_setting: ItemSetting::Item(1),
                is_clear: true,
                eeprom,
                buttons: AllButton::No
            },
            Local {
                display,
                led,
                exti_pin: exti_pin.fan1_rpm,
                timer_11,
                buffer: adc_dma_buffer,
                btn_minus_async: button.btn_minus,
                btn_ok_async: button.btn_ok,
                btn_plus_async: button.btn_plus,
            },
        )
    }

    // pub struct MyDisplay<DI: DisplayInterface> {
    //     inner: mipidsi::Display<SPIInterfaceNoCS<DI, ErasedPin<Output>>, ST7735s, ErasedPin<Output>>,
    //     backlight_pin: ErasedPin<Output>,
    //     fx_params: FXParams,
    // }

    #[task(local = [display, draw_static: bool = true], shared = [menu, data, settings, item_setting, is_clear], priority = 1)]
    async fn display_menu_task(cx: display_menu_task::Context) {
        // let display = unsafe { cx.shared.display.lock(|d| &mut *d.get()) };
        let display = cx.local.display;
        let mut shared = cx.shared;

        let mut screen: Screens<Display, DisplayError> = StartScreen::default().into();
        screen.draw_init(display);

        loop {
            (
                &mut shared.menu,
                &mut shared.is_clear,
            )
                .lock(|menu, is_clear| {
                    match menu {
                        Menu::Main => {
                            *cx.local.draw_static = true;
                            let data = shared.data.lock(|data| {
                                core::mem::replace(data, Data::new())
                            });

                            screen = Screens::Main(MainScreen::new(data, *is_clear));
                        }
                        Menu::Fan(fan) => {
                            *cx.local.draw_static = true;
                            (&mut shared.settings, &mut shared.item_setting).lock(|settings, item_setting| {
                                screen = Screens::Setting(SettingScreen::new(
                                    settings.fans[*fan - 1].clone(),
                                    *fan,
                                    item_setting.clone(),
                                    *is_clear
                                ));
                            });
                        }
                    }
                    *is_clear = false;
                });
            // if *cx.local.draw_static {
            //     *cx.local.draw_static = false;
              
            //     info!("draw_static");
            // }
            screen.draw_static(display);
            screen.draw_init(display);
            Mono::delay(50.millis()).await;
        }
    }

    #[task(shared = [buttons, eeprom, settings, item_setting, menu, is_clear], priority = 2)]
    async fn button_menu_task(mut cx: button_menu_task::Context) {

        loop {
            (
                &mut cx.shared.buttons, 
                &mut cx.shared.is_clear, 
                &mut cx.shared.menu, 
                &mut cx.shared.item_setting,
                &mut cx.shared.settings
            )
            .lock(|buttons, is_clear, menu, item_setting, settings| {
                match buttons {
                    AllButton::No => {},
                    AllButton::Minus(minus) => {
                        match minus {
                            ButtonState::ShortPress => {
                                match menu {
                                Menu::Fan(fan) => match item_setting {
                                        ItemSetting::Item(item) => {
                                            settings.fans[*fan - 1].items[*item - 1].0 -= 1;
                                        }
                                    },
                                    Menu::Main => {}
                                }
                                
                            },
                            ButtonState::LongPress => {},
                            ButtonState::LongPressDuration(_) => {
                                match menu {
                                    Menu::Fan(fan) => match item_setting {
                                        ItemSetting::Item(item) => {
                                            let setting =
                                                &mut settings.fans[*fan - 1].items[*item - 1].0;
                                            if *setting > 1 {
                                                *setting -= 1;
                                            }
                                        }
                                    },
                                    Menu::Main => {}
                                }
                            },
                        }
                    }
                    AllButton::Ok(ok) => {
                        match ok {
                            ButtonState::ShortPress => {
                                match item_setting {
                                    ItemSetting::Item(item) => {
                                        *item += 1;
                                        if *item > 8 {
                                            *item = 1;
                                        }
                                        *item_setting = ItemSetting::Item(*item)
                                }
                                }
                            }
                            ButtonState::LongPress => {
                                *is_clear = true;
                            
                                match menu {
                                    Menu::Main => *menu = Menu::Fan(1),
                                    Menu::Fan(fan) => {
                                        *item_setting = ItemSetting::Item(1);
                                        let prev_fan = *fan;
                                        if *fan < 4 {
                                            *fan += 1;
                                            *menu = Menu::Fan(*fan)
                                        } else {
                                            *fan = 1;
                                            *menu = Menu::Main
                                        }
                                    }
                                }
                            }
                            ButtonState::LongPressDuration(_) => {
                               
                            },
                        }
                    }
                    AllButton::Plus(plus) => {
                        match plus {
                            ButtonState::ShortPress => {
                                match menu {
                                    Menu::Fan(fan) => match item_setting {
                                        ItemSetting::Item(item) => {
                                            settings.fans[*fan - 1].items[*item - 1].0 += 1;
                                        }
                                    },
                                    Menu::Main => {}
                                }
                            },
                            ButtonState::LongPress => {},
                            ButtonState::LongPressDuration(_) => {
                                match menu {
                                    Menu::Fan(fan) => match item_setting {
                                        ItemSetting::Item(item) => {
                                            let setting =
                                                &mut settings.fans[*fan - 1].items[*item - 1].0;
    
                                            if *setting < 99 {
                                                *setting += 1;
                                            }
                                        }
                                    },
                                    Menu::Main => {}
                                }
                            },
                        }
                    }
                }
                *buttons = AllButton::No;
            });
            
            Mono::delay(10.millis()).await;
        }
    }

    #[task(local = [btn_minus_async, btn_plus_async, btn_ok_async], shared = [buttons, eeprom, settings, item_setting, menu, is_clear], priority = 2)]
    async fn button_task(mut cx: button_task::Context) {
        let button_task::SharedResources {
            eeprom,
            mut settings,
            mut buttons,
            mut item_setting,
            mut menu,
            mut is_clear,
            __rtic_internal_marker,
        } = cx.shared;

        // Button configuration
        let button_config = ButtonConfig::new(
            MyDuration::millis(20),
            MyDuration::millis(1),
            MyDuration::millis(500),
            ButtonMode::PullUp,
            100,
        );

        let mut btn_minus = Button::new(cx.local.btn_minus_async, button_config);
        let mut btn_plus = Button::new(cx.local.btn_plus_async, button_config);
        let mut btn_ok = Button::new(cx.local.btn_ok_async, button_config);

        loop {
            match select3(btn_minus.update(), btn_ok.update(), btn_plus.update()).await {
                Either3::First(minus) => {
                    buttons.lock(|buttons| {
                        match minus {
                            ButtonEvent::ShortPress(_) => *buttons = AllButton::Minus(ButtonState::ShortPress),
                            ButtonEvent::LongPress => *buttons = AllButton::Minus(ButtonState::LongPress),
                            ButtonEvent::LongPressDuration(d) => *buttons = AllButton::Minus(ButtonState::LongPressDuration(d)),
                            ButtonEvent::Released => {}
                        }
                    });
                }
                Either3::Second(ok) => {
                    buttons.lock(|buttons| {
                        match ok {
                            ButtonEvent::ShortPress(_) => *buttons = AllButton::Ok(ButtonState::ShortPress),
                            ButtonEvent::LongPress => *buttons = AllButton::Ok(ButtonState::LongPress),
                            ButtonEvent::LongPressDuration(d) => *buttons = AllButton::Ok(ButtonState::LongPressDuration(d)),
                            ButtonEvent::Released => {}
                        }
                    });
                }
                Either3::Third(plus) => {
                    buttons.lock(|buttons| {
                        match plus {
                            ButtonEvent::ShortPress(_) => *buttons = AllButton::Plus(ButtonState::ShortPress),
                            ButtonEvent::LongPress => *buttons = AllButton::Plus(ButtonState::LongPress),
                            ButtonEvent::LongPressDuration(d) => *buttons = AllButton::Plus(ButtonState::LongPressDuration(d)),
                            ButtonEvent::Released => {}
                        }
                    });
                }
            }
        }


        // loop {
        //     match select3(btn_minus.update(), btn_ok.update(), btn_plus.update()).await {
        //         Either3::First(minus) => match minus {
        //             ButtonEvent::ShortPress(_) => {
        //                 (&mut settings, &mut item_setting, &mut menu).lock(
        //                     |settings, item_setting, menu| match menu {
        //                         Menu::Fan(fan) => match item_setting {
        //                             ItemSetting::Item(item) => {
        //                                 settings.fans[*fan - 1].items[*item - 1].0 -= 1;
        //                             }
        //                         },
        //                         Menu::Main => {}
        //                     },
        //                 );
        //             }
        //             ButtonEvent::LongPress => {}
        //             ButtonEvent::LongPressDuration(_) => {
        //                 (&mut settings, &mut item_setting, &mut menu).lock(
        //                     |settings, item_setting, menu| match menu {
        //                         Menu::Fan(fan) => match item_setting {
        //                             ItemSetting::Item(item) => {
        //                                 let setting =
        //                                     &mut settings.fans[*fan - 1].items[*item - 1].0;
        //                                 if *setting > 1 {
        //                                     *setting -= 1;
        //                                 }
        //                             }
        //                         },
        //                         Menu::Main => {}
        //                     },
        //                 );
        //             }
        //             ButtonEvent::Released => {}
        //         },
        //         Either3::Second(ok) => match ok {
        //             ButtonEvent::ShortPress(_) => {
        //                 (item_setting).lock(|item_setting| match item_setting {
        //                     ItemSetting::Item(item) => {
        //                         *item += 1;
        //                         if *item > 8 {
        //                             *item = 1;
        //                         }
        //                         *item_setting = ItemSetting::Item(*item)
        //                     }
        //                 });
        //             }
        //             ButtonEvent::LongPress => {
        //                 (&mut is_clear, &mut menu, &mut item_setting).lock(
        //                     |is_clear, menu, item_setting| {
        //                         *is_clear = true;

        //                         match menu {
        //                             Menu::Main => *menu = Menu::Fan(1),
        //                             Menu::Fan(fan) => {
        //                                 *item_setting = ItemSetting::Item(1);
        //                                 if *fan < 4 {
        //                                     *fan += 1;
        //                                     *menu = Menu::Fan(*fan)
        //                                 } else {
        //                                     *fan = 1;
        //                                     *menu = Menu::Main
        //                                 }
        //                             }
        //                         }
        //                     },
        //                 );
        //             }
        //             ButtonEvent::LongPressDuration(_) => {}
        //             ButtonEvent::Released => {}
        //         },
        //         Either3::Third(plus) => match plus {
        //             ButtonEvent::ShortPress(_) => {
        //                 (&mut settings, &mut item_setting, &mut menu).lock(
        //                     |settings, item_setting, menu| match menu {
        //                         Menu::Fan(fan) => match item_setting {
        //                             ItemSetting::Item(item) => {
        //                                 settings.fans[*fan - 1].items[*item - 1].0 += 1;
        //                             }
        //                         },
        //                         Menu::Main => {}
        //                     },
        //                 );
        //             }
        //             ButtonEvent::LongPress => {}
        //             ButtonEvent::LongPressDuration(_) => {
        //                 (&mut settings, &mut item_setting, &mut menu).lock(
        //                     |settings, item_setting, menu| match menu {
        //                         Menu::Fan(fan) => match item_setting {
        //                             ItemSetting::Item(item) => {
        //                                 let setting =
        //                                     &mut settings.fans[*fan - 1].items[*item - 1].0;

        //                                 if *setting < 99 {
        //                                     *setting += 1;
        //                                 }
        //                             }
        //                         },
        //                         Menu::Main => {}
        //                     },
        //                 );
        //             }
        //             ButtonEvent::Released => {}
        //         },
        //     }
        // }
    }

   

    #[task(priority = 2)]
    async fn time_task(_ctx: time_task::Context) {
        loop {
            cortex_m::interrupt::free(|_| unsafe { GLOBAL_TIMER_COUNTER += TIMER_PERIOD });
            Mono::delay(1.millis()).await;
        }
    }

    #[task(shared = [data], local = [t: u16 = 20], priority = 2)]
    async fn test_task_temp(_ctx: test_task_temp::Context) {}

    #[task(shared = [data], local = [r: u16 = 1100], priority = 2)]
    async fn test_task_rpm(mut cx: test_task_rpm::Context) {
        loop {
            *cx.local.r -= 1;

            if *cx.local.r == 0 {
                *cx.local.r = 1100;
            }

            if *cx.local.r == 950 {
                *cx.local.r = 120;
            }

            cx.shared.data.lock(|data| {
                data[0].rpm = *cx.local.r;
            });

            Mono::delay(40.millis()).await;
        }
    }

    #[task(binds = TIM1_TRG_COM_TIM11, local = [timer_11, a: u32 = 0], shared = [transfer, frequency, interrupt_count], priority = 3)]
    fn timer_rpm_task(mut cx: timer_rpm_task::Context) {
        if cx.local.timer_11.flags().contains(Flag::Update) {
            cx.local.timer_11.clear_flags(Flag::Update)
        }

        cx.shared.transfer.lock(|transfer| {
            // transfer.start(|adc| {
            //     adc.start_conversion();
            //     // info!("conversion");
            // });
        });

        // *cx.local.a += 1;

        // if *cx.local.a % 100_000 == 0 {
        //     info!("LOCAL: {}", *cx.local.a);
        // }

        // let count = *cx.shared.interrupt_count;
        // *cx.shared.interrupt_count = 0;

        // cx.shared.frequency.lock(|frequency| {
        //     *frequency = count;
        //     // info!("{}", *frequency);
        // });

        // let count = cx.shared.interrupt_count.lock(|interrupt_count| {
        //     cx.shared.frequency.lock(|frequency| {
        //         *frequency = *interrupt_count;
        //         info!("{}", *frequency);
        //     });
        //     *interrupt_count = 0;

        // });
    }

    #[task(binds = EXTI15_10, local = [exti_pin], shared = [frequency, interrupt_count], priority = 3)]
    fn fan1_rpm_task(cx: fan1_rpm_task::Context) {
        *cx.shared.interrupt_count += 1;

        cx.local.exti_pin.clear_interrupt_pending_bit();
    }

    #[task(shared = [transfer], priority = 2)]
    async fn adc_start_task(mut cx: adc_start_task::Context) {
        loop {
            // cx.shared.transfer.lock(|transfer| {
            //     transfer.start(|adc| {
            //         adc.start_conversion();
            //     });
            // });
            Mono::delay(500.millis()).await;
        }
    }

    #[task(binds = DMA2_STREAM0, shared = [transfer, data], local = [buffer, a: u32 = 0], priority = 3)]
    fn dma(cx: dma::Context) {
        let mut shared = cx.shared;
        let local = cx.local;

        let (buffer, sample_to_millivolts) = shared.transfer.lock(|transfer| {
            let (buffer, _) = transfer
                .next_transfer(local.buffer.take().unwrap())
                .unwrap();

            let sample_to_millivolts = transfer.peripheral().make_sample_to_millivolts();

            (buffer, sample_to_millivolts)
        });

        let fan1_raw_adc = buffer[0];
        let fan2_raw_adc = buffer[1];

        *local.buffer = Some(buffer);

        let voltage_fan1 = sample_to_millivolts(fan1_raw_adc);
        let voltage_fan2 = sample_to_millivolts(fan2_raw_adc);

        *local.a += 1;

        shared.data.lock(|data| {
            data[1].rpm = voltage_fan1;
        });

        if *local.a % 4_000 == 0 {
            info!("LOCAL: {}", *local.a);
            info!("fan1: {}, fan2: {}", voltage_fan1, voltage_fan2);
        }
        // info!("fan1: {}, fan2: {}", voltage_fan1, voltage_fan2);
      
    }

    #[idle(local = [led])]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            // cx.local.led.toggle();
            rtic::export::wfi();
        }
    }
}
