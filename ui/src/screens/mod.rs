pub mod main;
pub mod setting;

use enum_dispatch::enum_dispatch;


#[allow(clippy::large_enum_variant)]
#[enum_dispatch]
pub enum Screens<DT: AppDrawTarget<E>, E: Debug> {
    Boot(MainScreen<DT, E>),
}


