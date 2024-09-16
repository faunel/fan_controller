use crate::ADC_BUFFER;
use core::ops::{Deref, DerefMut};
use ntc::Ntc;

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
    ntc: Ntc,
    temp: [u8; 4],
    rpm: [u16; 4],
    filter_temp: [f64; 4],
    filter_rpm: [f64; 4],
    smoothing_coefficient: f64,
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
    pub fn new(ema_window_size: u16, ntc: Ntc) -> Self {
        Data {
            ntc,
            smoothing_coefficient: 2.0 / (f64::from(ema_window_size) + 1.0),
            temp: [0; 4],
            rpm: [0; 4],
            filter_temp: [0.0; 4],
            filter_rpm: [0.0; 4],
        }
    }

    pub fn set_temp(&mut self, adc_values: &[u16; 4]) {
        for (ind, adc_value) in adc_values.iter().enumerate() {
            let temperature = self.ntc.get_temperature(adc_value);

            self.temp[ind] = temperature.map_or(99, |temp| {
                let temp = self.ema_temp(ind, temp);
                let temp = (temp + 0.5) as u8;

                if temp < 100 && temp > 0 {
                    temp
                } else {
                    99
                }
            });
        }
    }

    pub fn set_rpm(&mut self, impulses: &[u16; 4]) {
        for (ind, imp) in impulses.iter().enumerate() {
            let rpm = *imp / 2;
            let rpm = self.ema_rpm(ind, rpm as f64);

            self.rpm[ind] = (rpm + 0.5) as u16;
        }
    }

    pub fn get_temp(&self) -> &[u8; 4] {
        &self.temp
    }

    pub fn get_rpm(&self) -> &[u16; 4] {
        &self.rpm
    }

    fn ema_temp(&mut self, ind: usize, value: f64) -> f64 {
        Self::ema(&mut self.filter_temp, ind, value, self.smoothing_coefficient)
    }

    fn ema_rpm(&mut self, ind: usize, value: f64) -> f64 {
        Self::ema(&mut self.filter_rpm, ind, value, self.smoothing_coefficient)
    }

    fn ema(filter: &mut [f64; 4], ind: usize, value: f64, smoothing_coefficient: f64) -> f64 {
        if filter[ind] == 0.0 {
            filter[ind] = value;
        }

        filter[ind] += (value - filter[ind]) * smoothing_coefficient;

        filter[ind]
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
