// #![allow(unused)]

use crate::default_settings::DEFAULT_SETTINGS;
use defmt::info;
use eeprom24x::{Eeprom24x, SlaveAddr};
use heapless::Vec;
use monotonic::prelude::*;
use stm32f4xx_hal::{i2c::I2c, pac::I2C2};

type Eeprom = Eeprom24x<I2c<I2C2>, eeprom24x::page_size::B64, eeprom24x::addr_size::TwoBytes, eeprom24x::unique_serial::No>;

#[derive(Debug, Clone)]
pub struct Settings {
    pub fans: Vec<SettingFan, 4>,
}

#[derive(Debug, Default, Clone)]
pub struct SettingFan {
    pub thresold: Vec<Thresolds, 4>,
}

#[derive(Debug, Default, Clone)]
pub struct Thresolds {
    pub temp: Items,
    pub pwm: Items,
}

#[derive(Debug, Default, Clone)]
pub struct Items {
    pub data: u16,
    pub address: u32,
}

pub struct EEPROM {
    eeprom: Eeprom,
}

enum Modify {
    Increment,
    Decrement,
}

#[allow(clippy::new_without_default)]
impl Settings {
    #[must_use]
    pub fn new() -> Self {
        let mut current_address: u32 = 0;

        let fans: Vec<SettingFan, 4> = (0..4).map(|_| SettingFan::new(&mut current_address)).collect();

        Settings { fans }
    }

    pub fn increment_logic(&mut self, fan: &usize, selector: &usize) {
        let current_setting = self.get(fan, selector);
        let is_temperature = *selector % 2 != 0;
        let threshold = if is_temperature { 80 } else { 100 };

        if *current_setting < threshold {
            if *selector >= 7 {
                self.modify(fan, selector, Modify::Increment);
            } else {
                let last_setting = self.get(fan, &(*selector + 2));
                if *current_setting < last_setting - 1 {
                    self.modify(fan, selector, Modify::Increment);
                }
            }
        }
    }

    pub fn decrement_logic(&mut self, fan: &usize, selector: &usize) {
        let current_setting = self.get(fan, selector);
        if *current_setting > 0 {
            if *selector <= 2 {
                self.modify(fan, selector, Modify::Decrement);
            } else {
                let prev_setting = self.get(fan, &(*selector - 2));
                if *current_setting > prev_setting + 1 {
                    self.modify(fan, selector, Modify::Decrement);
                }
            }
        }
    }

    fn modify(&mut self, fan: &usize, selector: &usize, modify: Modify) {
        let s = match selector {
            1 => &mut self.fans[fan - 1].thresold[0].temp.data,
            2 => &mut self.fans[fan - 1].thresold[0].pwm.data,
            3 => &mut self.fans[fan - 1].thresold[1].temp.data,
            4 => &mut self.fans[fan - 1].thresold[1].pwm.data,
            5 => &mut self.fans[fan - 1].thresold[2].temp.data,
            6 => &mut self.fans[fan - 1].thresold[2].pwm.data,
            7 => &mut self.fans[fan - 1].thresold[3].temp.data,
            8 => &mut self.fans[fan - 1].thresold[3].pwm.data,
            _ => unreachable!(),
        };

        match modify {
            Modify::Increment => *s += 1,
            Modify::Decrement => *s -= 1,
        }
    }

    pub fn get(&self, fan: &usize, selector: &usize) -> &u16 {
        match selector {
            1 => &self.fans[fan - 1].thresold[0].temp.data,
            2 => &self.fans[fan - 1].thresold[0].pwm.data,
            3 => &self.fans[fan - 1].thresold[1].temp.data,
            4 => &self.fans[fan - 1].thresold[1].pwm.data,
            5 => &self.fans[fan - 1].thresold[2].temp.data,
            6 => &self.fans[fan - 1].thresold[2].pwm.data,
            7 => &self.fans[fan - 1].thresold[3].temp.data,
            8 => &self.fans[fan - 1].thresold[3].pwm.data,
            _ => unreachable!(),
        }
    }
}

impl SettingFan {
    pub fn new(current_address: &mut u32) -> Self {
        SettingFan {
            thresold: (0..4).map(|_| Thresolds::new(current_address)).collect(),
        }
    }
}

impl Thresolds {
    pub fn new(current_address: &mut u32) -> Self {
        let temp = Items { data: 0, address: *current_address };
        *current_address += 16;

        let pwm = Items { data: 0, address: *current_address };
        *current_address += 16;

        Thresolds { temp, pwm }
    }
}

impl EEPROM {
    #[must_use]
    pub fn new(i2c: I2c<I2C2>) -> Self {
        EEPROM {
            eeprom: Eeprom24x::new_24x256(i2c, SlaveAddr::default()),
        }
    }

    pub async fn save(&mut self, address: &u32, data: &u16) {
        let bytes = data.to_le_bytes();
        self.eeprom.write_page(*address, &bytes).unwrap();
        // field.data = data;
        Mono::delay(5.millis()).await;
        // self.delay_source.delay_ms(5);
    }

    pub async fn read(&mut self, address: &u32) -> u16 {
        let mut buffer = [0; 2];
        self.eeprom.read_data(*address, &mut buffer).unwrap();
        Mono::delay(100.micros()).await;
        // self.delay_source.delay_us(100);
        u16::from_le_bytes(buffer)
    }

    pub async fn default_settings(&mut self, settings: &mut Settings) {
        for fan in 0..4 {
            for (thresold, data) in DEFAULT_SETTINGS.iter().enumerate() {
                #[allow(clippy::get_first)]
                let temp = data.get(0).unwrap();
                let pwm = data.get(1).unwrap();

                settings.fans[fan].thresold[thresold].temp.data = *temp;
                self.save(&settings.fans[fan].thresold[thresold].temp.address, temp).await;

                settings.fans[fan].thresold[thresold].pwm.data = *pwm;
                self.save(&settings.fans[fan].thresold[thresold].pwm.address, pwm).await;
            }
        }
    }

    pub async fn load_settings(&mut self, settings: &mut Settings) {
        for fan in 0..4 {
            for thresold in 0..4 {
                settings.fans[fan].thresold[thresold].temp.data = self.read(&settings.fans[fan].thresold[thresold].temp.address).await;
                settings.fans[fan].thresold[thresold].pwm.data = self.read(&settings.fans[fan].thresold[thresold].pwm.address).await;
            }
        }
    }

    pub async fn save_all(&mut self, settings: &mut Settings) {
        for fan in 0..4 {
            for thresold in 0..4 {
                let temp_address = &settings.fans[fan].thresold[thresold].temp.address;
                let temp_data = &settings.fans[fan].thresold[thresold].temp.data;
                if *temp_data != self.read(temp_address).await {
                    self.save(temp_address, temp_data).await;
                    info!("fan: {}, thresold: {}, data: {}", fan, thresold, temp_data);
                }

                let pwm_address = &settings.fans[fan].thresold[thresold].pwm.address;
                let pwm_data = &settings.fans[fan].thresold[thresold].pwm.data;
                if *pwm_data != self.read(pwm_address).await {
                    self.save(pwm_address, pwm_data).await;
                    info!("fan: {}, thresold: {}, data: {}", fan, thresold, pwm_data);
                }
            }
        }
    }
}
