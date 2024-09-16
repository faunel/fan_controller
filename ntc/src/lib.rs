#![no_std]
#![allow(unused)]

use libm::log;

#[derive(Clone, Copy)]
pub struct Ntc {
    adc_resolution: u16,
    b_value: u16,
    resistor: f64,
    thermistor: f64,
    nominal_temp: f64,
    smoothing_coefficient: f64,
    filter_value: f64,
}

impl Ntc {
    #[must_use]
    pub fn new() -> Self {
        Ntc::default()
    }

    fn set_resolution(&mut self, adc_resolution: u16) -> &mut Self {
        self.adc_resolution = 1 << adc_resolution;
        self
    }

    fn set_b_value(&mut self, b_value: u16) -> &mut Self {
        self.b_value = b_value;
        self
    }

    fn set_resistor(&mut self, resistor: f64) -> &mut Self {
        self.resistor = resistor;
        self
    }

    fn set_thermistor(&mut self, thermistor: f64) -> &mut Self {
        self.thermistor = thermistor;
        self
    }

    fn set_nominal_temp(&mut self, nominal_temp: f64) -> &mut Self {
        self.nominal_temp = nominal_temp;
        self
    }

    fn set_ema_window_size(&mut self, window_size: u16) -> &mut Self {
        self.smoothing_coefficient = 2.0 / (f64::from(window_size) + 1.0);
        self
    }

    pub fn get_temperature(&self, adc_value: &u16) -> Option<f64> {
        if *adc_value == 0 {
            return None;
        }

        let resistance_ratio = self.resistor / self.thermistor;
        let adc_ratio = f64::from(self.adc_resolution - 1) / f64::from(*adc_value) - 1.0;
        let resistance = resistance_ratio / adc_ratio;
        let temp_kelvin = 1.0 / ((log(resistance) / f64::from(self.b_value)) + 1.0 / (self.nominal_temp + 273.15));
        let temp_celsius = temp_kelvin - 273.15;

        Some(temp_celsius)
    }
}

impl Default for Ntc {
    fn default() -> Self {
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
}
