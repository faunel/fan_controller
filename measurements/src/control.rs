use core::cmp::Ordering;
use eeprom::eeprom::Settings;
use stm32f4xx_hal::{hal::pwm::SetDutyCycle, pac::TIM4, timer};

use crate::measure::Data;

type TimerType = (timer::PwmChannel<TIM4, 0>, timer::PwmChannel<TIM4, 1>, timer::PwmChannel<TIM4, 2>, timer::PwmChannel<TIM4, 3>);

pub struct Control {
    timer: TimerType,
    old_pwm: [u8; 4],
    current_pwm: [u8; 4],
}

impl Control {
    pub fn new(timer: TimerType) -> Self {
        Control {
            timer,
            old_pwm: [0; 4],
            current_pwm: [0; 4],
        }
    }

    pub fn run(&mut self, settings: &Settings, data: &Data) {
        for (ind_fan, fan) in settings.fans.iter().enumerate() {
            let temp = data.get_temp();
            let current_temp = temp[ind_fan];

            // Якщо температура менша ніж перший поріг, плавно зменшуємо PWM до нуля
            if current_temp < fan.thresold[0].temp.data as u8 {
                self.current_pwm[ind_fan] = self.current_pwm[ind_fan].saturating_sub(1);
            // Інакше температура більша за якийсь поріг. В циклі перевіряємо за який
            } else {
                for ind_thresold in 0..4 {
                    let set_temp_from = fan.thresold[ind_thresold].temp.data as u8;

                    let set_temp_to = if ind_thresold <= 2 {
                        fan.thresold[ind_thresold + 1].temp.data as u8
                    } else {
                        u8::MAX
                    };

                    // current_temp >= set_temp_from && current_temp < set_temp_to
                    if (set_temp_from..set_temp_to).contains(&current_temp) {
                        let set_pwm = fan.thresold[ind_thresold].pwm.data as u8;

                        match self.current_pwm[ind_fan].cmp(&set_pwm) {
                            // Якщо PWM потрібно більший чим поточний, збільшуємо його
                            Ordering::Less => self.current_pwm[ind_fan] += 1,
                            // Якщо PWM потрібно менший чим поточний, зменшуємо його
                            Ordering::Greater => self.current_pwm[ind_fan] -= 1,
                            Ordering::Equal => {}
                        }
                    }
                }
            }

            if self.old_pwm[ind_fan] != self.current_pwm[ind_fan] {
                // info!("fan: {}, pwm: {}", ind_fan, self.current_pwm[ind_fan]);
                match ind_fan {
                    0 => {
                        // info!("FAN: {}, PWM: {}", ind_fan, self.current_pwm[ind_fan]);
                        self.timer.0.set_duty_cycle_percent(self.current_pwm[ind_fan]).unwrap()
                    }
                    1 => {
                        // info!("FAN: {}, PWM: {}", ind_fan, self.current_pwm[ind_fan]);
                        self.timer.1.set_duty_cycle_percent(self.current_pwm[ind_fan]).unwrap()
                    }
                    2 => self.timer.2.set_duty_cycle_percent(self.current_pwm[ind_fan]).unwrap(),
                    3 => self.timer.3.set_duty_cycle_percent(self.current_pwm[ind_fan]).unwrap(),
                    _ => unreachable!(),
                }
                self.old_pwm[ind_fan] = self.current_pwm[ind_fan];
            }
        }
    }
}
