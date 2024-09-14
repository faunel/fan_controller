#![no_std]
#![allow(unused)]

use libm::log;

#[derive(Debug, Default)]
pub struct Ntc {
    adc_resolution: u16,
    b_value: u16,
    resistor: f32,
    thermistor: f32,
    nominal_temp: f32,
    smoothing_coefficient: f32,

    filter_value: f32,
}

impl Ntc {
    #[must_use]
    pub fn new() -> Self {
        Ntc {
            adc_resolution: 1 << 12,
            b_value: 3950,
            resistor: 10.0,
            thermistor: 10.0,
            nominal_temp: 25.0,
            smoothing_coefficient: 1.0,

            filter_value: 0.0,
        }
    }

    pub fn set_resolution(&mut self, adc_resolution: u16) -> &mut Self {
        self.adc_resolution = 1 << adc_resolution;
        self
    }

    pub fn set_b_value(&mut self, b_value: u16) -> &mut Self {
        self.b_value = b_value;
        self
    }

    pub fn set_resistor(&mut self, resistor: f32) -> &mut Self {
        self.resistor = resistor;
        self
    }

    pub fn set_thermistor(&mut self, thermistor: f32) -> &mut Self {
        self.thermistor = thermistor;
        self
    }

    pub fn set_nominal_temp(&mut self, nominal_temp: f32) -> &mut Self {
        self.nominal_temp = nominal_temp;
        self
    }

    pub fn set_ema_window_size(&mut self, window_size: u16) -> &mut Self {
        self.smoothing_coefficient = 2.0 / (f32::from(window_size) + 1.0);
        self
    }

    pub fn get_temperature(&mut self, adc_value: &u16) -> Option<u8> {
        if *adc_value == 0 {
            return None;
        }

        let resistance_ratio = self.resistor / self.thermistor;
        let adc_ratio = f32::from(self.adc_resolution - 1) / f32::from(*adc_value) - 1.0;
        let resistance = f64::from(resistance_ratio / adc_ratio);
        let temp_kelvin = 1.0 / ((log(resistance) as f32 / f32::from(self.b_value)) + 1.0 / (self.nominal_temp + 273.15));
        let temp_celsius = temp_kelvin - 273.15;

        let filtered_temperature = (self.ema(temp_celsius) + 0.5) as u8;

        Some(filtered_temperature)
    }

    fn ema(&mut self, value: f32) -> f32 {
        if self.filter_value == 0.0 {
            self.filter_value = value;
        }

        self.filter_value += (value - self.filter_value) * self.smoothing_coefficient;

        self.filter_value
    }
}
