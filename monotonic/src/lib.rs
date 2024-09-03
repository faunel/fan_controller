#![no_std]
pub mod prelude {
    use fugit::Duration;
    pub use rtic_monotonics::systick::prelude::*;
    pub type MyDuration = Duration<u32, 1, 10_000>;

    systick_monotonic!(Mono, 10000);
}
