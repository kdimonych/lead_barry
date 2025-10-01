// Display driver imports
mod data_model;
mod screen;
mod screen_animation;
mod screen_collection;
mod screen_voltage;
mod screen_welcome;

pub use crate::ui::data_model::DataModel;
pub use crate::ui::screen::Screen;
pub use crate::ui::screen_animation::AnimationScreen;
pub use crate::ui::screen_collection::ScreenCollection;
pub use crate::ui::screen_voltage::VoltageScreen;
pub use crate::ui::screen_welcome::WelcomeScreen;

use ssd1306::I2CDisplayInterface;
use ssd1306::Ssd1306Async;
use ssd1306::prelude::*;

use embassy_sync::watch::DynReceiver;
use embassy_time::Ticker;

use crate::units::TimeExt;
use defmt::*;

pub struct Ui<'a, ActiveScreen, I2cDevice, DisplaySize>
where
    DisplaySize: ssd1306::size::DisplaySizeAsync,
    ActiveScreen: Screen + Clone,
{
    display:
        Ssd1306Async<I2cDevice, DisplaySize, ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize>>,
    active_screen: ActiveScreen,
    state_watch_receiver: DynReceiver<'a, ActiveScreen>,
}

/// Marker struct for UI interface creation (zero-sized factory pattern)
#[derive(Debug, Copy, Clone)]
pub struct UiInterface(());

impl UiInterface {
    /// Creates a new UI instance with SSD1306 display
    ///
    /// This is a factory method that returns `Ui<Interface, Size>`, not `Self`.
    /// The marker struct pattern is intentionally used here for namespace organization.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<'a, I2cDevice, DisplaySize, ActiveScreen>(
        i2c_dev: I2cDevice,
        display_size: DisplaySize,
        state_watch_receiver: DynReceiver<'a, ActiveScreen>,
        initial_screen: ActiveScreen,
    ) -> Ui<'a, ActiveScreen, ssd1306::prelude::I2CInterface<I2cDevice>, DisplaySize>
    where
        I2cDevice: embedded_hal_async::i2c::I2c,
        DisplaySize: ssd1306::size::DisplaySizeAsync,
        ActiveScreen: Screen + Clone,
    {
        let interface = I2CDisplayInterface::new(i2c_dev);
        let disp = Ssd1306Async::new(
            interface,
            display_size,
            ssd1306::prelude::DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();
        Ui {
            display: disp,
            active_screen: initial_screen,
            state_watch_receiver,
        }
    }
}

impl<'a, ActiveScreen, I2cDevice, DisplaySize>
    Ui<'a, ActiveScreen, ssd1306::prelude::I2CInterface<I2cDevice>, DisplaySize>
where
    ActiveScreen: Screen + Clone,
    DisplaySize: ssd1306::size::DisplaySizeAsync,
    I2cDevice: embedded_hal_async::i2c::I2c,
{
    pub async fn init(&mut self) {
        self.display.init().await.unwrap();
        self.display.flush().await.unwrap();
    }

    fn switch_screen(&mut self) {
        if let Some(new_screen) = self.state_watch_receiver.try_changed() {
            info!("Switching to new screen");
            self.active_screen.exit(&mut self.display);
            self.active_screen = new_screen;
            self.active_screen.enter(&mut self.display);
        }
    }

    pub async fn draw_loop(&mut self) -> ! {
        // Frame rate ticker for 25 FPS
        let mut ticker = Ticker::every((1000 / 25).ms());

        self.active_screen.enter(&mut self.display);
        loop {
            self.switch_screen();
            self.active_screen.redraw(&mut self.display);
            self.display.flush().await.unwrap();
            ticker.next().await;
        }
    }
}
