// Display driver imports
mod data_model;
mod screen;

mod screens;
mod ui_interface;

pub use self::data_model::DataModel;
pub use self::screen::Screen;
use crate::global_types::I2c0DeviceType;

pub use self::screens::*;
pub use self::ui_interface::UiInterface;

pub type UiSharedState = self::ui_interface::UiSharedState<ScCollection>;
pub type UiRunner<'a> = self::ui_interface::UiRunner<
    'a,
    I2c0DeviceType<'a>,
    ssd1306::size::DisplaySize128x64,
    ScCollection,
>;
pub type UiControl<'a> = self::ui_interface::UiControl<'a, ScCollection>;
