use crate::{ADC_BUFFER, RPM_BUFFER};
use ntc::Ntc;

#[derive(Default)]
pub struct AdcMeasure {
    buffer: [[u16; ADC_BUFFER / 4]; 4],
    data: [u16; 4],
}

#[derive(Default)]
pub struct RpmData {
    is_active: bool,
    data: u16,
}

#[derive(Default)]
pub struct RpmState {
    index: usize,
    frequency_buffer: f32,
    previous_capture: u32,
}

pub struct Data {
    ntc: Ntc,
    temp: [u16; 4],
    rpm: [u16; 4],
    filter_temp: [f64; 4],
    filter_rpm: [f64; 4],
    temp_smoothing_coefficient: f64,
    rpm_smoothing_coefficient: f64,
    thresold: [u8; 4],
}

/// Конфігурація вимірювань
pub struct MeasureConfig {
    /// Вікно для фільтра температури
    /// (Cкільки вимірів потрібно щоб значення було відфільтроване і дорівнювало вимірюваному)
    pub temp_ema_window: u16,
    /// Вікно для фільтра обертів вентилятора
    /// (Cкільки вимірів потрібно щоб значення було відфільтроване і дорівнювало вимірюваному)
    pub rpm_ema_window: u16,
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
    pub fn new(ntc: Ntc, config: MeasureConfig) -> Self {
        Data {
            ntc,
            temp_smoothing_coefficient: 2.0 / (f64::from(config.temp_ema_window) + 1.0),
            rpm_smoothing_coefficient: 2.0 / (f64::from(config.rpm_ema_window) + 1.0),
            temp: [0; 4],
            rpm: [0; 4],
            filter_temp: [0.0; 4],
            filter_rpm: [0.0; 4],
            thresold: [0; 4],
        }
    }

    pub fn set_temp(&mut self, adc_values: &[u16; 4]) {
        for (ind, adc_value) in adc_values.iter().enumerate() {
            let temperature = self.ntc.get_temperature(adc_value);

            self.temp[ind] = temperature.map_or(99, |temp| {
                let temp = self.ema_temp(ind, temp);
                let temp = (temp + 0.5) as u16;

                if temp < 100 && temp > 0 {
                    temp
                } else {
                    100
                }
            });
        }
    }

    pub fn set_rpm(&mut self, rpm: &[u16; 4]) {
        for (ind, rpm) in rpm.iter().enumerate() {
            let rpm = self.ema_rpm(ind, *rpm as f64);
            let rpm = (rpm + 0.5) as u16;
            if rpm < 10_000 {
                self.rpm[ind] = rpm;
            }
        }
    }

    pub fn set_thresold(&mut self, thresold: &[u8; 4]) {
        self.thresold = *thresold;
    }

    pub fn get_thresold(&self) -> &[u8; 4] {
        &self.thresold
    }

    pub fn get_temp(&self) -> &[u16; 4] {
        &self.temp
    }

    pub fn get_rpm(&self) -> &[u16; 4] {
        &self.rpm
    }

    fn ema_temp(&mut self, ind: usize, value: f64) -> f64 {
        Self::ema(&mut self.filter_temp, ind, value, self.temp_smoothing_coefficient)
    }

    fn ema_rpm(&mut self, ind: usize, value: f64) -> f64 {
        Self::ema(&mut self.filter_rpm, ind, value, self.rpm_smoothing_coefficient)
    }

    fn ema(filter: &mut [f64; 4], ind: usize, value: f64, smoothing_coefficient: f64) -> f64 {
        if filter[ind] == 0.0 {
            filter[ind] = value;
        }

        filter[ind] += (value - filter[ind]) * smoothing_coefficient;

        filter[ind]
    }
}

impl RpmState {
    pub fn new() -> [Self; 4] {
        [RpmState::default(), RpmState::default(), RpmState::default(), RpmState::default()]
    }

    pub fn set_prev_capture(&mut self, previous_capture: u32) {
        self.previous_capture = previous_capture;
    }

    pub fn get_prev_capture(&self) -> u32 {
        self.previous_capture
    }

    pub fn calculate_rpm(&mut self, freq: f32) -> Option<u16> {
        let index = self.index;
        self.frequency_buffer += freq;

        if index == RPM_BUFFER - 1 {
            let rpm = ((self.frequency_buffer * 30.0) as usize / RPM_BUFFER) as u16;
            self.frequency_buffer = 0.0;
            self.index = 0;
            return Some(rpm);
        }

        self.index += 1;

        None
    }

    // fn calculate_rpm(&self, fan: usize) -> u16 {
    //     let buffer =self.frequency_buffer[fan];
    //     let sum: f32 = buffer.iter().sum();
    //     (sum as usize / buffer.len()) as u16 * 30
    // }
}

impl RpmData {
    pub fn new() -> [Self; 4] {
        [RpmData::default(), RpmData::default(), RpmData::default(), RpmData::default()]
    }

    pub fn set_rpm(&mut self, rpm: u16) {
        self.data = rpm;
    }

    pub fn clear_rpm(&mut self) {
        self.data = 0;
    }

    pub fn get_rpm(&self) -> u16 {
        self.data
    }

    pub fn is_active_fan(&self) -> bool {
        self.is_active
    }

    pub fn set_active_fan(&mut self) {
        self.is_active = true;
    }

    pub fn clear_active_fans(&mut self) {
        self.is_active = false;
    }
}
