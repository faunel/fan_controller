#![no_std]

const FREQ_MONO: u32 = 100_000;

pub mod prelude {
    pub use rtic_monotonics::systick::prelude::*;
    use fugit::Duration;
    use crate::FREQ_MONO;
    pub type MyDuration = Duration<u32, 1, FREQ_MONO>;

    systick_monotonic!(Mono, FREQ_MONO);
}
