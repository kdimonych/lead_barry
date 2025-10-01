use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};

use crate::ui::DataModel;
use crate::ui::Screen;

/// Example screen that draws a simple welcome message
#[derive(Clone)]
pub struct VoltageScreen {
    voltage: &'static DataModel<f32>,
    voltage_cache: f32,
}

impl VoltageScreen {
    pub fn new(voltage: &'static DataModel<f32>) -> Self {
        Self {
            voltage,
            voltage_cache: 0.0,
        }
    }
    pub fn update_voltage(&mut self) {
        if let Ok(v) = self.voltage.try_lock() {
            self.voltage_cache = *v;
        }
    }
}

impl Screen for VoltageScreen {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Update the voltage reading from data model
        self.update_voltage();

        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();

        // Draw a welcome message
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();

        // Draw voltage
        let mut buffer = heapless::String::<32>::new();
        core::fmt::write(&mut buffer, format_args!("V: {:.2} V", self.voltage_cache)).ok();

        Text::with_baseline(&buffer, Point::new(10, 31), text_style, Baseline::Middle)
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
