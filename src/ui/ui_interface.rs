// Display driver imports
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, SendFuture, Sender};
use embassy_time::Ticker;

use crate::ui::screen::Screen;

use ssd1306::I2CDisplayInterface;
use ssd1306::Ssd1306Async;
use ssd1306::prelude::*;

use crate::units::TimeExt;
use defmt::*;

pub struct UiSharedState<ScreenSet> {
    screen_channel: Channel<CriticalSectionRawMutex, ScreenSet, 1>,
    active_screen: Option<ScreenSet>,
}

impl<ScreenSet> UiSharedState<ScreenSet> {
    pub fn new() -> Self {
        Self {
            screen_channel: Channel::new(),
            active_screen: None,
        }
    }
}

pub struct UiRunner<'a, SharedI2cDevice, DisplaySize, ScreenSet> {
    i2c_dev: Option<SharedI2cDevice>,
    display_size: Option<DisplaySize>,
    screen_receiver: Receiver<'a, CriticalSectionRawMutex, ScreenSet, 1>,
    active_screen: &'a mut Option<ScreenSet>,
}

pub struct UiControl<'a, ScreenSet> {
    screen_sender: Sender<'a, CriticalSectionRawMutex, ScreenSet, 1>,
}

impl<'a, ScreenSet> UiControl<'a, ScreenSet> {
    fn new(screen_sender: Sender<'a, CriticalSectionRawMutex, ScreenSet, 1>) -> Self {
        Self { screen_sender }
    }

    pub fn switch(
        &self,
        new_screen: ScreenSet,
    ) -> SendFuture<'a, CriticalSectionRawMutex, ScreenSet, 1> {
        debug!("Send switching to new screen ...");
        self.screen_sender.send(new_screen)
    }
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
    pub fn new<'a, SharedI2cDevice, DisplaySize, ScreenSet>(
        i2c_dev: SharedI2cDevice,
        display_size: DisplaySize,
        state: &'a mut UiSharedState<ScreenSet>,
        initial_screen: Option<ScreenSet>,
    ) -> (
        UiControl<'a, ScreenSet>,
        UiRunner<'a, SharedI2cDevice, DisplaySize, ScreenSet>,
    )
    where
        SharedI2cDevice: embedded_hal_async::i2c::I2c,
        DisplaySize: ssd1306::size::DisplaySizeAsync,
    {
        state.active_screen = initial_screen;
        (
            UiControl::new(state.screen_channel.sender()),
            UiRunner {
                i2c_dev: Some(i2c_dev),
                display_size: Some(display_size),
                screen_receiver: state.screen_channel.receiver(),
                active_screen: &mut state.active_screen,
            },
        )
    }
}

impl<'a, SharedI2cDevice, DisplaySize, ScreenSet>
    UiRunner<'a, SharedI2cDevice, DisplaySize, ScreenSet>
where
    DisplaySize: ssd1306::size::DisplaySizeAsync + Copy,
    SharedI2cDevice: embedded_hal_async::i2c::I2c,
{
    pub async fn run(&mut self) -> !
    where
        ScreenSet: Screen,
    {
        let i2c_dev = self.i2c_dev.take().expect("I2C device already taken");
        let display_size = self
            .display_size
            .take()
            .expect("Display size already taken");

        let interface = I2CDisplayInterface::new(i2c_dev);
        let mut display: Ssd1306Async<
            I2CInterface<SharedI2cDevice>,
            DisplaySize,
            ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize>,
        > = Ssd1306Async::new(
            interface,
            display_size,
            ssd1306::prelude::DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        debug!("Initializing the display ...");
        display.init().await.unwrap_or_else(|e| {
            error!("Init error: {:?}", e);
        });
        display.flush().await.unwrap_or_else(|e| {
            error!("Flush error: {:?}", e);
        });

        // Frame rate ticker for 25 FPS
        let mut ticker = Ticker::every((1000 / 25).ms());

        if let Some(active_screen) = self.active_screen {
            active_screen.enter(&mut display);
        }

        loop {
            if let Ok(mut new_screen) = self.screen_receiver.try_receive() {
                debug!("Switching to new screen ...");
                if let Some(old_screen) = self.active_screen {
                    old_screen.exit(&mut display);
                }

                new_screen.enter(&mut display);
                self.active_screen.replace(new_screen);

                debug!("Switching to new screen complete");
            } else if let Some(active_screen) = self.active_screen {
                active_screen.redraw(&mut display);
            }

            display.flush().await.unwrap_or_else(|e| {
                error!("Flush error: {:?}", e);
            });
            ticker.next().await;
        }
    }
}
