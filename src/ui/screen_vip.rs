use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Styled},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::DataModel;
use crate::ui::Screen;

#[derive(Clone, PartialEq)]
pub enum BaseUnits {
    Volts,
    Amps,
    Watts,
}

/// Example screen that draws a simple welcome message
#[derive(Clone)]
pub struct VIPScreen {
    voltage: &'static DataModel<f32>,
    voltage_cache: f32,
    base_unit: BaseUnits,
    unit_prefix: &'static str,

    //Internals
    character_style: MonoTextStyle<'static, BinaryColor>,
    text_style: TextStyle,
    rectangle: Styled<Rectangle, PrimitiveStyle<BinaryColor>>,
}

const fn unit(base_unit: &BaseUnits) -> &'static str {
    match base_unit {
        BaseUnits::Volts => "V",
        BaseUnits::Amps => "A",
        BaseUnits::Watts => "W",
    }
}

fn prefix(value: f32) -> (&'static str, f32) {
    let abs_value = value.abs();
    if abs_value >= 1_000_000.0 {
        ("M", value / 1_000_000.0)
    } else if abs_value >= 1_000.0 {
        ("k", value / 1_000.0)
    } else if abs_value >= 1.0 {
        ("", value)
    } else if abs_value >= 0.001 {
        ("m", value * 1_000.0)
    } else if abs_value >= 0.000_001 {
        ("u", value * 1_000_000.0)
    } else {
        ("n", value * 1_000_000_000.0)
    }
}

impl VIPScreen {
    pub fn new(voltage: &'static DataModel<f32>, base_unit: BaseUnits) -> Self {
        let (unit_prefix, _) = prefix(0.0);
        let character_style = MonoTextStyleBuilder::new()
            .font(&FONT_10X20)
            .text_color(BinaryColor::On)
            .build();

        let text_style = TextStyleBuilder::new()
            .baseline(Baseline::Middle)
            .alignment(Alignment::Center)
            .build();

        let style = PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .build();
        let rectangle = Rectangle::new(Point::new(5, 5), Size::new(118, 54)).into_styled(style);

        Self {
            voltage,
            voltage_cache: 0.0,
            base_unit,
            unit_prefix,
            character_style,
            text_style,
            rectangle,
        }
    }
    pub fn update_voltage(&mut self) {
        if let Ok(v) = self.voltage.try_lock() {
            let (unit_prefix, v) = prefix(*v);
            self.voltage_cache = v;
            self.unit_prefix = unit_prefix;
        }
    }
}

impl Screen for VIPScreen {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Update the voltage reading from data model
        self.update_voltage();

        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();

        // Draw voltage
        let mut buffer = heapless::String::<32>::new();

        core::fmt::write(
            &mut buffer,
            format_args!(
                "{:.3}{}{}",
                self.voltage_cache,
                self.unit_prefix,
                unit(&self.base_unit)
            ),
        )
        .ok();

        Text::with_text_style(
            &buffer,
            Point::new(64, 32),
            self.character_style,
            self.text_style,
        )
        .draw(draw_target)
        .ok();

        // Draw a rectangle border
        self.rectangle.draw(draw_target).ok();
    }
}
