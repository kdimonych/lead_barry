use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder},
    text::{Baseline, Text},
};

use crate::ui::Screen;

/// Example screen that draws animated graphics
#[derive(Clone)]
pub struct AnimationScreen {
    frame: u32,
}

impl AnimationScreen {
    pub fn new() -> Self {
        Self { frame: 0 }
    }
}

impl Screen for AnimationScreen {
    fn draw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();

        self.frame = self.frame.wrapping_add(1);

        // Draw animated circle
        let radius = 5 + (self.frame / 10) % 15;
        let x = 64 + ((self.frame as i32 * 2) % 40) - 20;
        let y = 32;

        let style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .build();

        Circle::new(Point::new(x - radius as i32, y - radius as i32), radius * 2)
            .into_styled(style)
            .draw(draw_target)
            .ok();

        // Draw frame counter
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        let mut buffer = heapless::String::<16>::new();
        core::fmt::write(&mut buffer, format_args!("Frame: {}", self.frame)).ok();

        Text::with_baseline(&buffer, Point::new(5, 10), text_style, Baseline::Top)
            .draw(draw_target)
            .ok();
    }
}
