use crate::default_settings;
use eeprom24x::{Eeprom24x, SlaveAddr};
use heapless::Vec;
use stm32f4xx_hal::{
    i2c::I2c,
    pac::{self, I2C2},
    prelude::*,
    timer,
};

type Eeprom = Eeprom24x<
    I2c<I2C2>,
    eeprom24x::page_size::B64,
    eeprom24x::addr_size::TwoBytes,
    eeprom24x::unique_serial::No,
>;

#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub fans: Vec<SettingFan, 4>,
}

#[derive(Debug, Default, Clone)]
pub struct SettingFan {
    pub items: Vec<(u16, u32), 8>,
}

// #[derive(Debug)]
// pub struct Thresholds {
//     pub temp: (u16, u32),
//     pub pwm: (u16, u32),
// }

pub struct EEPROM {
    eeprom: Eeprom,
    delay_source: timer::Delay<pac::TIM10, 1000000>,
}

#[allow(clippy::new_without_default)]
impl Settings {
    pub fn new() -> Self {
        let mut current_address: u32 = 0;

        let fans: Vec<SettingFan, 4> = (0..4)
            .map(|_| SettingFan::new(&mut current_address))
            .collect();

        Settings { fans }
    }

    pub fn get(&self, field: (u16, u32)) -> u16 {
        field.0
    }
}

impl SettingFan {
    pub fn new(current_address: &mut u32) -> Self {
        let items: Vec<(u16, u32), 8> = (0..8)
            .map(|_| {
                *current_address += 16;
                (0, *current_address)
            })
            .collect();

        SettingFan { items }
    }
    // pub fn new(current_address: &mut u32) -> Self {
    //     let items: Vec<(u16, u32), 4> =
    //         (0..4).map(|_| Thresholds::new(current_address)).collect();

    //     SettingFan { items }
    // }
}

// impl Thresholds {
//     pub fn new(current_address: &mut u32) -> Self {
//         let temp = (0, *current_address);
//         *current_address += 16;

//         let pwm = (0, *current_address);
//         *current_address += 16;

//         Thresholds { temp, pwm }
//     }

//     pub fn get_temp(&self) -> u16 {
//         self.temp.0
//     }

//     pub fn get_pwm(&self) -> u16 {
//         self.pwm.0
//     }

//     pub fn set_temp(&mut self, temp: u16) {
//         self.temp.0 = temp;
//     }

//     pub fn set_pwm(&mut self, pwm: u16) {
//         self.pwm.0 = pwm;
//     }
// }

impl EEPROM {
    pub fn new(i2c: I2c<I2C2>, delay_source: timer::Delay<pac::TIM10, 1000000>) -> Self {
        EEPROM {
            eeprom: Eeprom24x::new_24x256(i2c, SlaveAddr::default()),
            delay_source,
        }
    }

    pub fn save(&mut self, field: &mut (u16, u32), data: u16) {
        let bytes = data.to_le_bytes();
        self.eeprom.write_page(field.1, &bytes).unwrap();
        field.0 = data;
        self.delay_source.delay_ms(5);
    }

    pub fn read(&mut self, field: (u16, u32)) -> u16 {
        let mut buffer = [0; 2];
        self.eeprom.read_data(field.1, &mut buffer).unwrap();

        self.delay_source.delay_us(100);
        u16::from_le_bytes(buffer)
    }

    pub fn default_settings(&mut self, settings: &mut Settings) {
        self.save(
            &mut settings.fans[0].items[0],
            default_settings::FAN1_THRESOLD_1_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[0].items[1],
            default_settings::FAN1_THRESOLD_1_PWM,
        );
        self.save(
            &mut settings.fans[0].items[2],
            default_settings::FAN1_THRESOLD_2_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[0].items[3],
            default_settings::FAN1_THRESOLD_2_PWM,
        );
        self.save(
            &mut settings.fans[0].items[4],
            default_settings::FAN1_THRESOLD_3_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[0].items[5],
            default_settings::FAN1_THRESOLD_3_PWM,
        );
        self.save(
            &mut settings.fans[0].items[6],
            default_settings::FAN1_THRESOLD_4_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[0].items[7],
            default_settings::FAN1_THRESOLD_4_PWM,
        );

        self.save(
            &mut settings.fans[1].items[0],
            default_settings::FAN2_THRESOLD_1_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[1].items[1],
            default_settings::FAN2_THRESOLD_1_PWM,
        );
        self.save(
            &mut settings.fans[1].items[2],
            default_settings::FAN2_THRESOLD_2_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[1].items[3],
            default_settings::FAN2_THRESOLD_2_PWM,
        );
        self.save(
            &mut settings.fans[1].items[4],
            default_settings::FAN2_THRESOLD_3_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[1].items[5],
            default_settings::FAN2_THRESOLD_3_PWM,
        );
        self.save(
            &mut settings.fans[1].items[6],
            default_settings::FAN2_THRESOLD_4_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[1].items[7],
            default_settings::FAN2_THRESOLD_4_PWM,
        );

        self.save(
            &mut settings.fans[2].items[0],
            default_settings::FAN3_THRESOLD_1_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[2].items[1],
            default_settings::FAN3_THRESOLD_1_PWM,
        );
        self.save(
            &mut settings.fans[2].items[2],
            default_settings::FAN3_THRESOLD_2_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[2].items[3],
            default_settings::FAN3_THRESOLD_2_PWM,
        );
        self.save(
            &mut settings.fans[2].items[4],
            default_settings::FAN3_THRESOLD_3_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[2].items[5],
            default_settings::FAN3_THRESOLD_3_PWM,
        );
        self.save(
            &mut settings.fans[2].items[6],
            default_settings::FAN3_THRESOLD_4_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[2].items[7],
            default_settings::FAN3_THRESOLD_4_PWM,
        );

        self.save(
            &mut settings.fans[3].items[0],
            default_settings::FAN4_THRESOLD_1_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[3].items[1],
            default_settings::FAN4_THRESOLD_1_PWM,
        );
        self.save(
            &mut settings.fans[3].items[2],
            default_settings::FAN4_THRESOLD_2_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[3].items[3],
            default_settings::FAN4_THRESOLD_2_PWM,
        );
        self.save(
            &mut settings.fans[3].items[4],
            default_settings::FAN4_THRESOLD_3_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[3].items[5],
            default_settings::FAN4_THRESOLD_3_PWM,
        );
        self.save(
            &mut settings.fans[3].items[6],
            default_settings::FAN4_THRESOLD_4_TEMPERATURE,
        );
        self.save(
            &mut settings.fans[3].items[7],
            default_settings::FAN4_THRESOLD_4_PWM,
        );
    }

    pub fn load_settings(&mut self, settings: &mut Settings) {
        for fan in 0..4 {
            for thresold in 0..8 {
                settings.fans[fan].items[thresold].0 =
                    self.read(settings.fans[fan].items[thresold]);
            }
        }
    }
}
