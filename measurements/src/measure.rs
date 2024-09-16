use core::ops::{Deref, DerefMut};

use ntc::Ntc;

use crate::ADC_BUFFER;

#[derive(Default)]
pub struct ImpulsesRaw([u16; 4]);

#[derive(Default)]
pub struct ImpulsesComplete([u16; 4]);

#[derive(Default)]
pub struct AdcMeasure {
    buffer: [[u16; ADC_BUFFER / 4]; 4],
    data: [u16; 4],
}

pub struct Data {
    temp: [u8; 4],
    rpm: [u16; 4],
    ntc: [Ntc; 4],
}

impl ImpulsesRaw {
    pub fn new() -> Self {
        ImpulsesRaw::default()
    }

    pub fn add_raw(&mut self, index: usize) {
        self[index] = self[index].saturating_add(1);
    }
}

impl ImpulsesComplete {
    pub fn new() -> Self {
        ImpulsesComplete::default()
    }

    pub fn set(&mut self, index: usize, fan: &u16) {
        self[index] = *fan;
    }
}

impl AdcMeasure {
    pub fn new() -> Self {
        AdcMeasure::default()
    }

    pub fn split_channels(&mut self, buffer: &[u16; ADC_BUFFER]) -> &[u16; 4] {
        for (i, &value) in buffer.iter().enumerate() {
            match i % 4 {
                0 => self.buffer[0][i / 4] = value,
                1 => self.buffer[1][i / 4] = value,
                2 => self.buffer[2][i / 4] = value,
                3 => self.buffer[3][i / 4] = value,
                _ => unreachable!(),
            }
        }
        self.average()
    }

    fn average(&mut self) -> &[u16; 4] {
        for (ind, buf) in self.buffer.iter().enumerate() {
            let sum: u32 = buf.iter().map(|&x| x as u32).sum();
            self.data[ind] = (sum / buf.len() as u32) as u16;
        }
        &self.data
    }
}

impl Data {
    #[must_use]
    pub fn new() -> Self {
        Data::default()
    }

    pub fn set_temp(&mut self, adc_values: &[u16; 4]) {
        for (ind, adc_value) in adc_values.iter().enumerate() {
            let temperature = self.ntc[ind].set_ema_window_size(25).get_temperature(adc_value);

            if let Some(mut temp) = temperature {
                if temp > 99 {
                    temp = 99;
                }
                self.temp[ind] = temp;
            }
        }
    }

    pub fn set_rpm(&mut self, impulses: &[u16; 4]) {
        for (ind, imp) in impulses.iter().enumerate() {
            self.rpm[ind] = *imp / 2;
        }
    }

    pub fn get_temp(&self) -> &[u8; 4] {
        &self.temp
    }

    pub fn get_rpm(&self) -> &[u16; 4] {
        &self.rpm
    }
}

// Реалізація трейтів Deref та DerefMut
impl Deref for ImpulsesRaw {
    type Target = [u16; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ImpulsesRaw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Реалізація трейтів Deref та DerefMut
impl Deref for ImpulsesComplete {
    type Target = [u16; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ImpulsesComplete {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for Data {
    fn default() -> Self {
        Data {
            temp: [0; 4],
            rpm: [0; 4],
            ntc: [Ntc::new(), Ntc::new(), Ntc::new(), Ntc::new()],
        }
    }
}
