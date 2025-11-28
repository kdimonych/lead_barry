use common::any_string::AnyString;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
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

fn len_in_chars(word: &str) -> usize {
    word.chars().count()
}

fn split_whitespace_once(text: &str) -> (&str, &str) {
    if let Some((first, rest)) = text.split_once(char::is_whitespace) {
        (first, rest.trim_start())
    } else {
        (text, &text[text.len()..])
    }
}

fn split_line_once(text: &str) -> (&str, &str) {
    if let Some((first, rest)) = text.split_once(['\n', '\r']) {
        (first, rest.trim_start())
    } else {
        (text, &text[text.len()..])
    }
}

/// Fill the line with words from the message until it reaches the maximum length or runs out of words.
/// Returns the length of the line and the remaining part of the message.
/// If a word is too long to fit in an empty line, it will be truncated and "..." will be added.
/// If a word is too long to fit in a non-empty line, it will be left for the next line.
fn fill_with_words<'a>(
    line: &mut heapless::String<MESSAGE_SIZE>,
    msg: &'a str,
) -> (usize, &'a str) {
    let mut rest = msg;
    let mut old_rest = msg;
    let mut line_len = len_in_chars(line.as_ref());

    while line.len() <= MESSAGE_LINE_LENGTH {
        let (word, rest_msg) = split_whitespace_once(rest);
        rest = rest_msg;
        if word.is_empty() {
            break;
        }
        let word_len = len_in_chars(word);
        let space_len = if line_len > 0 { 1 } else { 0 };
        if line_len + word_len + space_len <= MESSAGE_LINE_LENGTH {
            if space_len > 0 {
                // Add space before the word if it's not the first word in the line
                line.push(' ').ok();
            }
            line.push_str(word).ok();
            line_len += space_len + word_len;
        } else if line.is_empty() {
            // In case this is new line and the word is too long, we need to split it
            let (part_len, part, _) = try_split_at_utf8(word, MESSAGE_LINE_LENGTH - space_len - 3);
            if space_len > 0 {
                // Add space before the word if it's not the first word in the line
                line.push(' ').ok();
            }
            line.push_str(part).ok();
            line.push_str("...").ok();
            line_len += space_len + part_len + 3;
            break;
        } else {
            // In case the line is not empty and the word is too long, we need a new line, so just return the old rest
            rest = old_rest;
            break;
        }

        old_rest = rest;
    }
    (line_len, rest)
}

// TODO: Cover with tests
fn split_message_into_lines(message: &str) -> heapless::Vec<heapless::String<MESSAGE_SIZE>, 3> {
    let mut lines: heapless::Vec<heapless::String<MESSAGE_SIZE>, 3> = heapless::Vec::new();
    for _ in 0..lines.capacity() {
        lines.push(heapless::String::new()).ok();
    }

    let mut target_lines = lines.iter_mut();
    let mut rest = message;

    while let Some(mut current_line) = target_lines.next() {
        let (mut msg_line, msg_rest) = split_line_once(rest);
        rest = msg_rest;

        while !msg_line.is_empty() {
            let (_, line_rest) = fill_with_words(current_line, msg_line);
            msg_line = line_rest;

            let Some(new_current_line) = target_lines.next() else {
                return lines;
            };
            current_line = new_current_line;
        }

        if rest.is_empty() {
            break;
        }
    }
    // Truncate empty lines at the end
    lines.truncate(lines.iter().filter(|line| !line.is_empty()).count());

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
