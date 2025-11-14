pub mod actions;
mod button_controller;

//pub use button_controller::ButtonEvent;

const BUTTON_EVENT_QUEUE_SIZE: usize = 8;
const BUTTONS_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, defmt::Format)]
pub enum Buttons {
    Yellow,
    Blue,
}

pub type ButtonEvent = button_controller::ButtonEvent<Buttons>;

pub type ButtonControllerState =
    button_controller::ButtonControllerState<Buttons, BUTTONS_COUNT, BUTTON_EVENT_QUEUE_SIZE>;
pub type ButtonController<'a> =
    button_controller::ButtonController<'a, Buttons, BUTTON_EVENT_QUEUE_SIZE>;
pub type ButtonControllerBuilder =
    button_controller::ButtonControllerBuilder<BUTTONS_COUNT, Buttons>;
pub type ButtonControllerRunner<'a> =
    button_controller::ButtonControllerRunner<'a, Buttons, BUTTONS_COUNT, BUTTON_EVENT_QUEUE_SIZE>;
