#![no_std]

use libm::log;

#[derive(Clone, Copy)]
pub struct Ntc {
    adc_resolution: u16,
    b_value: u16,
    resistor: f64,
    thermistor: f64,
    nominal_temp: f64,
}

impl Ntc {
    #[must_use]
    pub fn new() -> Self {
        Ntc::default()
    }

    /// Роздільна здатність АЦП в бітах
    pub fn set_resolution(&mut self, adc_resolution: u16) -> &mut Self {
        self.adc_resolution = 1 << adc_resolution;
        self
    }

    /// Характеристика B терморезистора
    pub fn set_b_value(&mut self, b_value: u16) -> &mut Self {
        self.b_value = b_value;
        self
    }

    /// Резистор в kOm, який послідовно з'єднаний з терморезистором
    pub fn set_resistor(&mut self, resistor: f64) -> &mut Self {
        self.resistor = resistor;
        self
    }

    /// Терморезистор в kOm
    pub fn set_thermistor(&mut self, thermistor: f64) -> &mut Self {
        self.thermistor = thermistor;
        self
    }

    /// Температура при якій терморезистор має свій номінальний опір (зазвичай це 25 градусів)
    pub fn set_nominal_temp(&mut self, nominal_temp: f64) -> &mut Self {
        self.nominal_temp = nominal_temp;
        self
    }

    /// Отримання значення температури в градусах цельсія
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
        }
    }
}
