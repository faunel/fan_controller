#![no_std]

pub mod control;
pub mod measure;

pub const ADC_BUFFER: usize = 64;
pub const RPM_BUFFER: usize = 8;
