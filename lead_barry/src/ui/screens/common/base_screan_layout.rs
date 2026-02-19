use super::screan_constants::*;

use common::any_string::AnyString;
use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_8X13_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

pub const TITLE_LENGTH: usize = 15;
pub const TITLE_SIZE: usize = TITLE_LENGTH * 4; // UTF-8 can be up to 4 bytes per char

// Title layout constants
pub const TITLE_TEXT_POSITION: Point = Point::new(SCREEN_MIDDLE_X, (TITLE_HEIGHT / 2) as i32);
pub const TITLE_HEIGHT: u32 = 16;
pub const TITLE_BOX: Rectangle = Rectangle::new(SCREEN_TL, Size::new(SCREEN_WIDTH, TITLE_HEIGHT));

//Message frame layout constants
pub const MESSAGE_FRAME_BORDER: Rectangle = Rectangle::new(
    Point::new(SCREEN_TL.x, SCREEN_TL.y + (TITLE_HEIGHT as i32)),
    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT - TITLE_HEIGHT),
);
pub const MESSAGE_FRAME_THICKNESS: u32 = 1;

pub const TITLE_BOX_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::Off)
    .stroke_width(0)
    .fill_color(BinaryColor::On)
    .build();

pub const FRAME_BORDER_STYLE_BUILDER: PrimitiveStyleBuilder<BinaryColor> =
    PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(MESSAGE_FRAME_THICKNESS);
pub const FRAME_BORDER_STYLE: PrimitiveStyle<BinaryColor> = FRAME_BORDER_STYLE_BUILDER.build();

pub const TITLE_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13_BOLD)
    .text_color(BinaryColor::Off)
    .build();

pub const TITLE_TEXT_STYLE_BUILDER: TextStyleBuilder = TextStyleBuilder::new()
    .baseline(Baseline::Middle)
    .alignment(Alignment::Center);
pub const TITLE_TEXT_STYLE: TextStyle = TITLE_TEXT_STYLE_BUILDER.build();

/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type TitleString<'a> = AnyString<'a, TITLE_SIZE>;

pub fn draw_base_screen_layout<D>(draw_target: &mut D)
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Draw title box
    TITLE_BOX
        .into_styled(TITLE_BOX_STYLE)
        .draw(draw_target)
        .ok();

    // Draw a frame border
    MESSAGE_FRAME_BORDER
        .into_styled(FRAME_BORDER_STYLE)
        .draw(draw_target)
        .ok();
}

pub fn draw_title_text<D>(draw_target: &mut D, title_str: &str)
where
    D: DrawTarget<Color = BinaryColor>,
{
    Text::with_text_style(
        title_str,
        TITLE_TEXT_POSITION,
        TITLE_CHARACTER_STYLE,
        TITLE_TEXT_STYLE,
    )
    .draw(draw_target)
    .ok();
}
