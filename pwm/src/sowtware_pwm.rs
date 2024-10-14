use embedded_hal::digital::OutputPin;
use monotonic::prelude::*;

#[derive(Debug)]
pub struct SowtwarePwm<Pin> {
    pin: Pin,
}

impl<Pin> SowtwarePwm<Pin> 
where
    Pin: OutputPin,
{
    pub fn new(pin: Pin) -> Self {
        SowtwarePwm { pin }
    }

    /// freq_hz: 1 - 255
    /// duty_cycle: 0 - 100
    pub async fn pwm_hz(&mut self, freq_hz: u8, duty_cycle: u8) {
        assert!(freq_hz > 0);

        let delay = (10_000 / freq_hz as u16) as u32;

        for counter in 0..100 {
            if counter >= duty_cycle {
                self.pin.set_low().unwrap();
            } else {
                self.pin.set_high().unwrap();
            }
            Mono::delay(delay.micros()).await;
        }
    }
}
