use stm32f4xx_hal::{
    pac::{self, TIM3, TIM5},
    timer,
};

pub struct PwmInputOne {
    pub timer: timer::PwmInput<TIM5>,
    pub data_dirty: Option<f32>,
    pub data: u8,
}

pub struct PwmInputTwo {
    pub timer: timer::PwmInput<TIM3>,
    pub data_dirty: Option<f32>,
    pub data: u8,
}

impl PwmInputOne {
    pub fn new(timer: timer::PwmInput<TIM5>) -> Self {
        PwmInputOne { timer, data_dirty: None, data: 0 }
    }

    pub fn set_dirty_duty_cycle(&mut self) {
        if self.timer.is_valid_capture() {
            self.data_dirty = Some(self.timer.get_duty_cycle());
        }
    }

    pub fn get_duty_cycle(&mut self) -> u8 {
        if let Some(val) = self.data_dirty.take() {
            (val + 0.5) as u8
        } else {
            let pin_state = unsafe { (*pac::GPIOA::ptr()).idr().read().idr0().bit_is_set() };
            if pin_state {
                100
            } else {
                0
            }
        }
    }
}

impl PwmInputTwo {
    pub fn new(timer: timer::PwmInput<TIM3>) -> Self {
        PwmInputTwo { timer, data_dirty: None, data: 0 }
    }

    pub fn set_dirty_duty_cycle(&mut self) {
        if self.timer.is_valid_capture() {
            self.data_dirty = Some(self.timer.get_duty_cycle());
        }
    }

    pub fn get_duty_cycle(&mut self) -> u8 {
        if let Some(val) = self.data_dirty.take() {
            (val + 0.5) as u8
        } else {
            let pin_state = unsafe { (*pac::GPIOB::ptr()).idr().read().idr4().bit_is_set() };

            if pin_state {
                100
            } else {
                0
            }
        }
    }
}
