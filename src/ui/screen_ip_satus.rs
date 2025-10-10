use crate::ui::Screen;
use defmt::Str;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};

pub enum IpState {
    GettingIp,
    IpAssigned,
}

trait Verb {
    fn str(&self) -> &str;
}

impl Verb for IpState {
    fn str(&self) -> &str {
        match self {
            IpState::GettingIp => "Getting IP...",
            IpState::IpAssigned => "My IP:",
        }
    }
}

/// Example screen that draws a simple welcome message
pub struct IpStatusScreen {
    ip: embassy_net::Ipv4Address,
    ip_state: IpState,
    animation_iteration: u32,
    try_count: u8,
    buffer: heapless::String<32>,
}

impl IpStatusScreen {
    pub const fn new(ip: embassy_net::Ipv4Address, ip_state: IpState, try_count: u8) -> Self {
        Self {
            ip,
            ip_state,
            animation_iteration: 0,
            try_count,
            buffer: heapless::String::new(),
        }
    }
}

impl Screen for IpStatusScreen {
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
            self.ip_state.str(),
            Point::new(10, 20),
            text_style,
            Baseline::Top,
        )
        .draw(draw_target)
        .ok();

        if let IpState::IpAssigned = self.ip_state {
            core::fmt::write(&mut self.buffer, format_args!("{}", self.ip)).ok();
            Text::with_baseline(
                self.buffer.as_str(),
                Point::new(10, 35),
                text_style,
                Baseline::Top,
            )
            .draw(draw_target)
            .ok();
        } else if self.try_count > 0 {
            core::fmt::write(&mut self.buffer, format_args!("({})", self.try_count)).ok();
            Text::with_baseline(
                self.buffer.as_str(),
                Point::new(10, 35),
                text_style,
                Baseline::Top,
            )
            .draw(draw_target)
            .ok();
        }

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
