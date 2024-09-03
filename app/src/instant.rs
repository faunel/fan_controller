use button_driver::InstantProvider;
use core::{ops::Sub, time::Duration};

/// This setting affects how fast a button can track a state change.
// Maximum resolution supported by the timer.
pub const TIMER_PERIOD: Duration = Duration::from_millis(1);

/// How much time has passed since the interrupt start?
pub static mut GLOBAL_TIMER_COUNTER: Duration = Duration::ZERO;

/// Retrieve the current time.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Instant {
    counter: Duration,
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, rhs: Instant) -> Self::Output {
        self.counter - rhs.counter
    }
}

impl InstantProvider<Duration> for Instant {
    fn now() -> Self {
        Instant {
            counter: cortex_m::interrupt::free(|_| unsafe { GLOBAL_TIMER_COUNTER }),
        }
    }
}
