use super::MyDuration;

pub(crate) const HOLD_DURATION_DELAY: u32 = 60;

/// [`Button`](super::Button) configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonConfig {
    /// Time the button should be down in order to count it as a press.
    pub debounce: MyDuration,
    /// Time between consecutive presses to count as a press in the same sequence instead of a new
    /// sequence.
    pub double_click: MyDuration,
    /// Time the button is held before a long press is detected.
    pub long_press: MyDuration,
    /// Button direction.
    pub mode: ButtonMode,
    //
    pub hold_duration_delay: u32,
}

impl ButtonConfig {
    /// Returns a new [ButtonConfig].
    #[must_use]
    pub fn new(debounce: MyDuration, double_click: MyDuration, long_press: MyDuration, mode: ButtonMode, hold_duration_delay: u32) -> Self {
        Self {
            debounce,
            double_click,
            long_press,
            mode,
            hold_duration_delay,
        }
    }
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            debounce: MyDuration::millis(10),
            double_click: MyDuration::millis(350),
            long_press: MyDuration::millis(1000),
            mode: ButtonMode::default(),
            hold_duration_delay: HOLD_DURATION_DELAY,
        }
    }
}

/// Button direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonMode {
    /// Button is connected to a pin with a pull-up resistor. Button pressed it logic 0.
    #[default]
    PullUp,
    /// Button is connected to a pin with a pull-down resistor. Button pressed it logic 1.
    PullDown,
}

impl ButtonMode {
    /// Is button connected to a pin with a pull-up resistor?
    #[must_use]
    pub const fn is_pullup(&self) -> bool {
        matches!(self, ButtonMode::PullUp)
    }

    /// Is button connected to a pin with a pull-down resistor?
    #[must_use]
    pub const fn is_pulldown(&self) -> bool {
        !self.is_pullup()
    }
}
