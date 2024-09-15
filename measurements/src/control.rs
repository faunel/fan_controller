use defmt::info;
use eeprom::eeprom::Settings;
use heapless::Vec;
use stm32f4xx_hal::{hal::pwm::SetDutyCycle, pac::TIM4, timer};

type TimerType = (timer::PwmChannel<TIM4, 0>, timer::PwmChannel<TIM4, 1>, timer::PwmChannel<TIM4, 2>, timer::PwmChannel<TIM4, 3>);

pub struct Control {
    timer: TimerType,
    old_pwm: Vec<u8, 4>,
}

impl Control {
    pub fn new(timer: TimerType) -> Self {
        Control {
            old_pwm: (0..4).map(|_| 0).collect(),
            timer,
        }
    }

    pub fn run(&mut self, temp: &Vec<u8, 4>, settings: &Settings) {
        for (ind_fan, fan) in settings.fans.iter().enumerate() {
            let mut set_pwm = 0;
            for (ind_thresold, thresold) in fan.thresold.iter().enumerate() {
                let set_temp = thresold.temp.data as u8;
                let current_temp = temp[ind_thresold];

                if current_temp >= set_temp {
                    set_pwm = thresold.pwm.data as u8;
                }
            }

            if self.old_pwm[ind_fan] != set_pwm {
                info!("fan: {}, pwm: {}", ind_fan, set_pwm);
                match ind_fan {
                    0 => self.timer.0.set_duty_cycle_percent(set_pwm).unwrap(),
                    1 => self.timer.1.set_duty_cycle_percent(set_pwm).unwrap(),
                    2 => self.timer.2.set_duty_cycle_percent(set_pwm).unwrap(),
                    3 => self.timer.3.set_duty_cycle_percent(set_pwm).unwrap(),
                    _ => unreachable!(),
                }
                self.old_pwm[ind_fan] = set_pwm;
            }
        }
    }
}
