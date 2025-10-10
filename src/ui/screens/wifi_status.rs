use crate::ui::Screen;
use defmt::Str;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};

#[derive(Clone)]
pub enum State {
    Disconnected,
    Connecting,
    Dhcp,
    Connected,
}

trait Verb {
    fn str(&self) -> &str;
}

impl Verb for State {
    fn str(&self) -> &str {
        match self {
            State::Disconnected => "Disconnected",
            State::Connecting => "Connecting to:",
            State::Dhcp => "Getting IP...",
            State::Connected => "Connected to:",
        }
    }
}

pub struct ScWifiStats {
    wifi_network_name: heapless::String<32>,
    wifi_state: State,
    animation_iteration: u32,
    try_count: u8,
    buffer: heapless::String<32>,
}

impl ScWifiStats {
    pub const fn new(
        wifi_network_name: heapless::String<32>,
        wifi_state: State,
        try_count: u8,
    ) -> Self {
        Self {
            wifi_network_name,
            wifi_state,
            animation_iteration: 0,
            try_count,
            buffer: heapless::String::new(),
        }
    }
}

impl Screen for ScWifiStats {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();

        // Draw a welcome message
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        Text::with_baseline(
            self.wifi_state.str(),
            Point::new(10, 20),
            text_style,
            Baseline::Top,
        )
        .draw(draw_target)
        .ok();

        let msg: &'_ str = if self.try_count > 0 {
            core::fmt::write(
                &mut self.buffer,
                format_args!("{}({})", self.wifi_network_name.as_str(), self.try_count),
            )
            .ok();
            self.buffer.as_str()
        } else {
            self.wifi_network_name.as_str()
        };

        Text::with_baseline(msg, Point::new(10, 35), text_style, Baseline::Top)
            .draw(draw_target)
            .ok();

        // Draw a rectangle border
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .build();

        Rectangle::new(Point::new(5, 5), Size::new(118, 54))
            .into_styled(style)
            .draw(draw_target)
            .ok();
    }
}
