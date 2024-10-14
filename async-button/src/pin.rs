use core::convert::Infallible;
use embedded_hal::digital::{ErrorType, InputPin};
use embedded_hal_async::digital::Wait;
use monotonic::prelude::*;

const DELAY: MyDuration = MyDuration::millis(1);

#[derive(Debug)]
pub struct WaitPin<P> {
    pin: P,
}

impl<P> WaitPin<P> {
    pub fn new(pin: P) -> Self {
        Self { pin }
    }
}

// Реалізація ErrorType для WaitPin
impl<P> ErrorType for WaitPin<P> {
    type Error = Infallible;
}

// Реалізація InputPin для WaitPin
impl<P> InputPin for WaitPin<P>
where
    P: InputPin<Error = Infallible>,
{
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.pin.is_high()
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.pin.is_low()
    }
}

// Реалізація Wait для WaitPin
impl<P> Wait for WaitPin<P>
where
    P: InputPin<Error = Infallible>,
{
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        while self.is_low()? {
            Mono::delay(DELAY).await;
        }
        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        while self.is_high()? {
            Mono::delay(DELAY).await;
        }
        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        while self.is_high()? {
            Mono::delay(DELAY).await;
        }
        // Чекаємо, поки пін стане високим
        while self.is_low()? {
            Mono::delay(DELAY).await;
        }
        Ok(())
    }

    /// pin to go high and then low again.
    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        while self.is_low()? {
            Mono::delay(DELAY).await;
        }
        // Чекаємо, поки пін стане низьким
        while self.is_high()? {
            Mono::delay(DELAY).await;
        }
        Ok(())
    }

    /// Wait for the pin to undergo any transition, i.e low to high OR high to low.
    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        let initial_state = self.is_high()?;
        loop {
            // Чекаємо на зміну стану
            if self.is_high()? != initial_state {
                break;
            }
            Mono::delay(DELAY).await;
        }
        Ok(())
    }
}
