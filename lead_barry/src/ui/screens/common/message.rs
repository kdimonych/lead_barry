use core::str::FromStr;

use common::any_string::AnyString;
use common::string_tools::*;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use super::base_screan_layout::*;
use super::screan_constants::*;
use crate::ui::{Screen, screens::common::base_screan_layout};

const MESSAGE_LINE_LENGTH: usize = 18;
const MESSAGE_LENGTH: usize = MESSAGE_LINE_LENGTH * 3; // Support for three lines of message
const MESSAGE_SIZE: usize = MESSAGE_LENGTH * 4; // UTF-8 can be up to 4 bytes per char

/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type MsgTitleString<'a> = base_screan_layout::TitleString<'a>;
/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type MessageString<'a> = AnyString<'a, MESSAGE_SIZE>;

pub trait TrMessage {
    fn title(&'_ self) -> MsgTitleString<'_>;
    fn message(&'_ self) -> MessageString<'_>;
}

pub struct ScMessageImpl<StatusT> {
    status: StatusT,
}

impl<StatusT> ScMessageImpl<StatusT>
where
    StatusT: TrMessage,
{
    pub const fn new(status: StatusT) -> Self {
        Self { status }
    }
}

impl<StatusT> Screen for ScMessageImpl<StatusT>
where
    StatusT: TrMessage,
{
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();
        draw_base_screen_layout(draw_target);

        let title = self.status.title();
        draw_title_text(draw_target, title.as_str());

        let message_str = self.status.message();
        let lines = split_message_into_lines(message_str.as_str());

        let mut lines_texts: heapless::Vec<Text<'_, MonoTextStyle<'static, BinaryColor>>, 3> =
            heapless::Vec::new();

        for line in &lines {
            let text = Text::with_text_style(
                line.as_str(),
                MESSAGE_TEXT_POSITION,
                MESSAGE_CHARACTER_STYLE,
                MESSAGE_TEXT_STYLE,
            );
            lines_texts.push(text).ok();
        }

        align_text_center_vertically(&mut lines_texts);

        //Draw the text
        for text_line in &lines_texts {
            text_line.draw(draw_target).ok();
        }
    }
}

fn align_text_center_vertically(
    text_list: &mut heapless::Vec<Text<'_, MonoTextStyle<'static, BinaryColor>>, 3>,
) {
    const SPACE: u32 = 2;

    if text_list.is_empty() {
        return;
    }

    let mut total_height: u32 = text_list
        .iter()
        .map(|text| text.bounding_box().size.height)
        .sum();
    total_height += if text_list.len() > 1 {
        (text_list.len() as u32 - 1) * SPACE
    } else {
        0
    };

    // Adjust the y position of each text to center them vertically
    let mut y_start = MESSAGE_TEXT_POSITION.y - ((total_height + 1) as i32 / 2);
    for text in text_list.iter_mut() {
        let text_box_height = text.bounding_box().size.height;
        let old_top_y = text.bounding_box().top_left.y;
        let y_offset = y_start - old_top_y;

        text.position.y += y_offset;
        y_start += text_box_height as i32 + SPACE as i32;
    }
}

fn try_split_at_utf8(word: &str, step: usize) -> (usize, &str, &str) {
    let mut i: usize = 0;
    while word.chars().next().is_some() && i < step {
        i += 1;
    }

    (i, &word[0..i], &word[i..])
}

// TODO: Cover with tests
fn split_message_into_lines(message: &str) -> heapless::Vec<heapless::String<MESSAGE_SIZE>, 3> {
    let mut lines: heapless::Vec<heapless::String<MESSAGE_SIZE>, 3> = heapless::Vec::new();

    let line_it = message.slice_by_lines(MESSAGE_LINE_LENGTH);
    for line in line_it {
        lines
            .push(heapless::String::from_str(line).unwrap_or_default())
            .ok();
    }
    lines
}

/* Constants */

//Message frame layout constants
const MESSAGE_FRAME_BORDER: Rectangle = Rectangle::new(
    Point::new(SCREEN_TL.x, SCREEN_TL.y + (TITLE_HEIGHT as i32)),
    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT - TITLE_HEIGHT),
);
const MESSAGE_TEXT_POSITION: Point = Point::new(
    MESSAGE_FRAME_BORDER.top_left.x + (MESSAGE_FRAME_BORDER.size.width as i32 / 2),
    MESSAGE_FRAME_BORDER.top_left.y + (MESSAGE_FRAME_BORDER.size.height as i32 / 2),
);

// Styles
const MESSAGE_TEXT_STYLE_BUILDER: TextStyleBuilder = TextStyleBuilder::new()
    .baseline(Baseline::Middle)
    .alignment(Alignment::Center);
const MESSAGE_TEXT_STYLE: TextStyle = MESSAGE_TEXT_STYLE_BUILDER.build();

// Fonts
const MESSAGE_CHARACTER_STYLE_BUILDER: MonoTextStyleBuilder<'static, BinaryColor> =
    MonoTextStyleBuilder::new()
        .font(&FONT_7X14_BOLD)
        .text_color(BinaryColor::On);
const MESSAGE_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> =
    MESSAGE_CHARACTER_STYLE_BUILDER.build();
