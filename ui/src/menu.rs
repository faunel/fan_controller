#[derive(Debug)]
pub enum Menu {
    Main,
    Fan(usize),
}

// #[derive(Debug)]
// pub enum Setting {

// }

pub enum AllButton {
    No,
    Minus(ButtonState),
    Ok(ButtonState),
    Plus(ButtonState),
}

pub enum ButtonState {
    ShortPress,
    LongPress,
    LongPressDuration(u32)
}
