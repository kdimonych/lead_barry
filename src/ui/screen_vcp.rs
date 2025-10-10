use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{
        Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Styled,
    },
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::DataModel;
use crate::ui::Screen;

// Layout constants
const FRAME_BORDER: Rectangle = Rectangle::new(Point::new(4, 4), Size::new(119, 55));
const VALUE_TEXT_POSITION: Point = Point::new(64, 32);

// Styles
const TEXT_FIELD_FRAME_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .fill_color(BinaryColor::Off)
    .stroke_color(BinaryColor::On) // Contrasting outline
    .stroke_width(2)
    .stroke_alignment(StrokeAlignment::Center)
    .build();
const CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_10X20)
    .text_color(BinaryColor::On)
    .build();
const VALUE_TEXT_STYLE: TextStyle = TextStyleBuilder::new()
    .baseline(Baseline::Middle)
    .alignment(Alignment::Center)
    .build();
const FRAME_BORDER_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .build();

#[derive(PartialEq)]
pub enum BaseUnits {
    Volts,
    Amps,
    Watts,
}

/// Example screen that draws a simple welcome message
pub struct VIPScreen {
    voltage: &'static DataModel<f32>,
    voltage_cache: f32,
    base_unit: BaseUnits,
    unit_prefix: &'static str,
}

const fn unit(base_unit: &BaseUnits) -> &'static str {
    match base_unit {
        BaseUnits::Volts => "V",
        BaseUnits::Amps => "A",
        BaseUnits::Watts => "W",
    }
}

// Determine appropriate SI prefix for a given value
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

        Self {
            voltage,
            voltage_cache: 0.0,
            base_unit,
            unit_prefix,
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

fn adaptive_precision_format<const N: usize>(
    buffer: &mut heapless::String<N>,
    value: f32,
    unit_prefix: &'static str,
    unit: &'static str,
) -> Result<(), core::fmt::Error> {
    let abs_value = value.abs();

    if abs_value < 1.0 {
        // 0.136578 -> 0.137 (3 decimal places)
        core::fmt::write(
            buffer,
            format_args!("{:.3}{:>2}{}", value, unit_prefix, unit),
        )?;
    } else if abs_value < 10.0 {
        // 1.36578 -> 1.366 (3 decimal places)
        core::fmt::write(
            buffer,
            format_args!("{:.3}{:>2}{}", value, unit_prefix, unit),
        )?;
    } else if abs_value < 100.0 {
        // 13.6578 -> 13.66 (2 decimal places)
        core::fmt::write(
            buffer,
            format_args!("{:.2}{:>2}{}", value, unit_prefix, unit),
        )?;
    } else if abs_value < 1000.0 {
        // 136.578 -> 136.6 (1 decimal place)
        core::fmt::write(
            buffer,
            format_args!("{:.1}{:>2}{}", value, unit_prefix, unit),
        )?;
    } else {
        // 136578 -> 136578 (0 decimal places, but could be scientific notation)
        core::fmt::write(
            buffer,
            format_args!("{:.0}{:>2}{}", value, unit_prefix, unit),
        )?;
    }
    Ok(())
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

        adaptive_precision_format(
            &mut buffer,
            self.voltage_cache,
            self.unit_prefix,
            unit(&self.base_unit),
        )
        .ok();

        let value_text = Text::with_text_style(
            &buffer,
            VALUE_TEXT_POSITION,
            CHARACTER_STYLE,
            VALUE_TEXT_STYLE,
        );
        let text_box = value_text.bounding_box().offset(2);

        let frame_y_mid = text_box.top_left.y + (text_box.size.height as i32) / 2;
        let text_box_right_side_x = text_box.top_left.x + text_box.size.width as i32;
        let text_box_bottom_side_y = text_box.top_left.y + text_box.size.height as i32;
        let left_corner = Point::new(text_box.top_left.x - 3, frame_y_mid);
        let right_corner = Point::new(text_box_right_side_x + 3, frame_y_mid);

        Polyline::new(&[
            Point::new(FRAME_BORDER.top_left.x, frame_y_mid),
            left_corner,
            Point::new(text_box.top_left.x, text_box.top_left.y),
            Point::new(text_box_right_side_x, text_box.top_left.y),
            right_corner,
            Point::new(
                FRAME_BORDER.top_left.x + FRAME_BORDER.size.width as i32,
                frame_y_mid,
            ),
        ])
        .into_styled(TEXT_FIELD_FRAME_STYLE)
        .draw(draw_target)
        .ok();

        Polyline::new(&[
            left_corner,
            Point::new(text_box.top_left.x, text_box_bottom_side_y),
            Point::new(text_box_right_side_x, text_box_bottom_side_y),
            right_corner,
        ])
        .into_styled(TEXT_FIELD_FRAME_STYLE)
        .draw(draw_target)
        .ok();

        value_text.draw(draw_target).ok();
        // Draw a frame border
        FRAME_BORDER
            .into_styled(FRAME_BORDER_STYLE)
            .draw(draw_target)
            .ok();
    }
}
