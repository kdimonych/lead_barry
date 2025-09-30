use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};

use crate::ui::Screen;

/// Example screen that draws a simple welcome message
#[derive(Clone)]
pub struct WelcomeScreen {
    counter: u32,
}

impl WelcomeScreen {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn increment(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }
}

impl Screen for WelcomeScreen {
    fn draw<D>(&mut self, draw_target: &mut D)
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

        Text::with_baseline("Welcome!", Point::new(10, 20), text_style, Baseline::Top)
            .draw(draw_target)
            .ok();

        // Draw counter
        let mut buffer = heapless::String::<32>::new();
        core::fmt::write(&mut buffer, format_args!("Count: {}", self.counter)).ok();

        Text::with_baseline(&buffer, Point::new(10, 35), text_style, Baseline::Top)
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
