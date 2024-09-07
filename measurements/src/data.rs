use heapless::Vec;
use ntc::Ntc;

#[derive(Default, Debug, Clone, Copy)]
pub struct Data {
    pub temp: u16,
    pub rpm: u16,
}

impl Data {
    pub fn new() -> Vec<Self, 4> {
        let mut data: Vec<Self, 4> = Vec::new();
        for _ in 1..=4 {
            data.push(Self::default()).unwrap();
        }
        data
    }


    pub fn set_temp(&mut self, temp: u16) {
        self.temp = temp;
    }

    pub fn set_rpm(&mut self, rpm: u16) {
        self.rpm = rpm;
    }

    pub fn get_temp(&self) -> u16 {
        self.temp
    }

    pub fn get_rpm(&self) -> u16 {
        self.rpm
    }
}
