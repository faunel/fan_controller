#![no_std]

use config::{ButtonConfig, ButtonMode};
use monotonic::prelude::*;
mod config;
mod pin;

pub mod prelude {
    pub use crate::{
        config::{ButtonConfig, ButtonMode},
        pin::WaitPin,
        Button, ButtonEvent,
    };
}

/// A generic button that asynchronously detects [`ButtonEvent`]s.
#[derive(Debug, Clone, Copy)]
pub struct Button<P> {
    pin: P,
    state: State,
    count: usize,
    hold_duration: u32, // Додаємо поле для зберігання часу утримання
    config: ButtonConfig,
}

#[derive(Debug, Clone, Copy)]
enum State {
    /// Initial state.
    Unknown,
    /// Debounced press.
    Pressed,
    /// The button was just released, waiting for more presses in the same sequence, or for the
    /// sequence to end.
    Released,
    /// Fully released state, idle.
    Idle,
    /// Waiting for the button to be released.
    PendingRelease,
}

/// Detected button events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    /// A sequence of 1 or more short presses.
    /// The number of short presses in the sequence.
    ShortPress(usize),
    /// A long press. This event is returned directly when the button is held for more than
    /// [`ButtonConfig::long_press`].
    LongPress,
    /// A long press with duration. This event is returned periodically while the button is held.
    LongPressDuration(u32),
}

impl<P> Button<P>
where
    P: embedded_hal_async::digital::Wait + embedded_hal::digital::InputPin,
{
    /// Creates a new button with the provided config.
    pub const fn new(pin: P, config: ButtonConfig) -> Self {
        Self {
            pin,
            state: State::Unknown,
            count: 0,
            hold_duration: 0,
            config,
        }
    }

    /// Updates the button and returns the detected event.
    ///
    /// Awaiting this blocks execution of the task until a [`ButtonEvent`] is detected so it should
    /// **not** be called from tasks where blocking for long periods of time is not desireable.
    pub async fn update(&mut self) -> ButtonEvent {
        loop {
            if let Some(event) = self.update_step().await {
                return event;
            }
        }
    }

    async fn update_step(&mut self) -> Option<ButtonEvent> {
        match self.state {
            State::Unknown => {
                if self.is_pin_pressed() {
                    self.state = State::Pressed;
                } else {
                    self.state = State::Idle;
                }
                None
            }

            State::Pressed => {
                match Mono::timeout_after(self.config.long_press, self.wait_for_release()).await {
                    Ok(()) => {
                        // Short press
                        self.debounce_delay().await;
                        if self.is_pin_released() {
                            self.state = State::Released;
                        }
                        None
                    }
                    Err(_) => {
                        // Long press detected
                        self.count = 0;
                        self.state = State::PendingRelease;
                        Some(ButtonEvent::LongPress)
                    }
                }
            }

            State::Released => {
                match Mono::timeout_after(self.config.double_click, self.wait_for_press()).await {
                    Ok(()) => {
                        // Continue sequence
                        self.debounce_delay().await;
                        if self.is_pin_pressed() {
                            self.count += 1;
                            self.state = State::Pressed;
                        }
                        None
                    }
                    Err(_) => {
                        // Sequence ended
                        let count = self.count;
                        self.count = 0;
                        self.state = State::Idle;
                        Some(ButtonEvent::ShortPress(count))
                    }
                }
            }

            State::Idle => {
                self.wait_for_press().await;
                self.debounce_delay().await;
                if self.is_pin_pressed() {
                    self.count = 1;
                    self.state = State::Pressed;
                }
                None
            }

            // State::PendingRelease => {
            //     self.wait_for_release().await;
            //     self.debounce_delay().await;
            //     if self.is_pin_released() {
            //         self.state = State::Idle;
            //     }
            //     None
            // }
            State::PendingRelease => {
                // Перевірка стану кнопки з паузою
                Mono::delay(self.config.hold_duration_delay.millis()).await;

                if self.is_pin_released() {
                    self.debounce_delay().await; // Додаємо перевірку дребезгу при відпусканні
                    if self.is_pin_released() {
                        self.state = State::Idle;
                        None
                    } else {
                        self.hold_duration += self.config.hold_duration_delay;
                        // Відправка події з оновленим часом утримання
                        Some(ButtonEvent::LongPressDuration(self.hold_duration))
                    }
                } else {
                    self.hold_duration += self.config.hold_duration_delay;
                    // Відправка події з оновленим часом утримання
                    Some(ButtonEvent::LongPressDuration(self.hold_duration))
                }
            }
        }
    }

    fn is_pin_pressed(&mut self) -> bool {
        self.pin.is_low().unwrap_or(self.config.mode.is_pulldown()) == self.config.mode.is_pullup()
    }

    fn is_pin_released(&mut self) -> bool {
        !self.is_pin_pressed()
    }

    async fn wait_for_release(&mut self) {
        match self.config.mode {
            ButtonMode::PullUp => self.pin.wait_for_high().await.unwrap_or_default(),
            ButtonMode::PullDown => self.pin.wait_for_low().await.unwrap_or_default(),
        }
    }

    async fn wait_for_press(&mut self) {
        match self.config.mode {
            ButtonMode::PullUp => self.pin.wait_for_low().await.unwrap_or_default(),
            ButtonMode::PullDown => self.pin.wait_for_high().await.unwrap_or_default(),
        }
    }

    async fn debounce_delay(&self) {
        Mono::delay(self.config.debounce).await;
    }
}
