use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_9X18_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::Screen;

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
    .font(&FONT_9X18_BOLD)
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

pub struct ScWifiAp {}

impl ScWifiAp {
    pub fn new() -> Self {
        Self {}
    }
}

impl Screen for ScWifiAp {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();

        let value_text = Text::with_text_style(
            "Lead Barry",
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
