// Display driver imports
mod screen;
mod screen_animation;
mod screen_welcome;

pub use crate::ui::screen::Screen;
pub use crate::ui::screen_animation::AnimationScreen;
pub use crate::ui::screen_welcome::WelcomeScreen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
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
    pub fn new<'a, I2cDevice, DisplaySize>(
        i2c_dev: I2cDevice,
        display_size: DisplaySize,
        state_watch_receiver: DynReceiver<'a, ScreenSet>,
    ) -> Ui<'a, ScreenSet, ssd1306::prelude::I2CInterface<I2cDevice>, DisplaySize>
    where
        I2cDevice: embedded_hal_async::i2c::I2c,
        DisplaySize: ssd1306::size::DisplaySizeAsync,
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
            active_screen: ScreenSet::Empty,
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

    pub async fn draw_once(&mut self) {
        if let Some(new_screen) = self.state_watch_receiver.try_changed() {
            info!("Switching to new screen");
            self.active_screen = new_screen;
        }
        self.active_screen.draw(&mut self.display);
        self.display.flush().await.unwrap();
    }

    pub async fn draw_loop(&mut self) {
        // Frame rate ticker for 25 FPS
        let mut ticker = Ticker::every((1000 / 25).ms());

        // Initial draw
        self.draw_once().await;

        loop {
            ticker.next().await;
            self.draw_once().await;
        }
    }
}

#[derive(Clone)]
pub enum ScreenSet {
    Welcome(WelcomeScreen),
    Animation(AnimationScreen),
    Empty,
}

impl Screen for ScreenSet {
    fn draw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScreenSet::Welcome(screen) => screen.draw(draw_target),
            ScreenSet::Animation(screen) => screen.draw(draw_target),
            ScreenSet::Empty => (),
        }
    }
}
