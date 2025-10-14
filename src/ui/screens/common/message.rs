use common::any_string::AnyString;
use core::str::SplitWhitespace;
use defmt::info;
use nalgebra::constraint;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{
        Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, line,
    },
    text::{self, Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::Screen;

const TITLE_LENGTH: usize = 15;
const TITLE_SIZE: usize = TITLE_LENGTH * 4; // UTF-8 can be up to 4 bytes per char
const MESSAGE_LINE_LENGTH: usize = 18;
const MESSAGE_LINE_SIZE: usize = MESSAGE_LINE_LENGTH * 4; // UTF-8 can be up to 4 bytes per char
const MESSAGE_LENGTH: usize = MESSAGE_LINE_LENGTH * 3; // Support for three lines of message
const MESSAGE_SIZE: usize = MESSAGE_LENGTH * 4; // UTF-8 can be up to 4 bytes per char

/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type MsgTitleString<'a> = AnyString<'a, TITLE_SIZE>;
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
        draw_main_screen_layout(draw_target);

        let title = self.status.title();
        Text::with_text_style(
            title.as_str(),
            TITLE_TEXT_POSITION,
            TITLE_CHARACTER_STYLE,
            TITLE_TEXT_STYLE,
        )
        .draw(draw_target)
        .ok();

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

        let mut text_box = align_text_center_vertically(&mut lines_texts);

        //Draw the text
        for text_line in &lines_texts {
            text_line.draw(draw_target).ok();
        }

        text_box = text_box.offset(2);
        let constraint = MESSAGE_FRAME_BORDER.offset(-(MESSAGE_FRAME_THICKNESS as i32) - 3);

        if text_box.size.width <= constraint.size.width
            && text_box.size.height <= constraint.size.height
        {
            // TODO: Move this to a function to avoid code duplication
            let frame_y_mid = text_box.top_left.y + (text_box.size.height as i32) / 2;
            let text_box_right_side_x = text_box.top_left.x + text_box.size.width as i32 - 1;
            let text_box_bottom_side_y = text_box.top_left.y + text_box.size.height as i32 - 1;
            let left_corner = Point::new(text_box.top_left.x - 3, frame_y_mid);
            let right_corner = Point::new(text_box_right_side_x + 3, frame_y_mid);

            Polyline::new(&[
                Point::new(
                    MESSAGE_FRAME_BORDER.top_left.x + (MESSAGE_FRAME_THICKNESS as i32),
                    frame_y_mid,
                ),
                left_corner,
                Point::new(text_box.top_left.x, text_box.top_left.y),
                Point::new(text_box_right_side_x, text_box.top_left.y),
                right_corner,
                Point::new(
                    MESSAGE_FRAME_BORDER.top_left.x + (MESSAGE_FRAME_BORDER.size.width as i32)
                        - 1
                        - (MESSAGE_FRAME_THICKNESS as i32),
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
        }
    }
}

fn align_text_center_vertically(
    text_list: &mut heapless::Vec<Text<'_, MonoTextStyle<'static, BinaryColor>>, 3>,
) -> Rectangle {
    const SPACE: u32 = 2;
    if text_list.is_empty() {
        return Rectangle::default();
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

    // Adjust the bounding box to include all texts
    let mut bounding_box = text_list.first().unwrap().bounding_box();
    for text in text_list.iter() {
        let text_box = text.bounding_box();
        bounding_box.top_left = bounding_box.top_left.component_min(text_box.top_left);
        bounding_box.size.width = bounding_box.size.width.max(text_box.size.width);
    }
    bounding_box.size.height = total_height;

    bounding_box
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
// Screen constants
const SCREEN_TL: Point = Point::new(0, 0);
const SCREEN_BR: Point = Point::new(127, 63);
const SCREEN_WIDTH: u32 = (SCREEN_BR.x - SCREEN_TL.x + 1) as u32;
const SCREEN_HEIGHT: u32 = (SCREEN_BR.y - SCREEN_TL.y + 1) as u32;
const SCREEN_MIDDLE_X: i32 = SCREEN_TL.x + (SCREEN_WIDTH / 2) as i32;

// Title layout constants
const TITLE_HEIGHT: u32 = 16;
const TITLE_BOX: Rectangle = Rectangle::new(SCREEN_TL, Size::new(SCREEN_WIDTH, TITLE_HEIGHT));
const TITLE_TEXT_POSITION: Point = Point::new(SCREEN_MIDDLE_X, (TITLE_HEIGHT / 2) as i32);

//Message frame layout constants
const MESSAGE_FRAME_BORDER: Rectangle = Rectangle::new(
    Point::new(SCREEN_TL.x, SCREEN_TL.y + (TITLE_HEIGHT as i32)),
    Size::new(SCREEN_WIDTH, SCREEN_HEIGHT - TITLE_HEIGHT),
);
const MESSAGE_FRAME_THICKNESS: u32 = 1;
const MESSAGE_TEXT_POSITION: Point = Point::new(
    MESSAGE_FRAME_BORDER.top_left.x + (MESSAGE_FRAME_BORDER.size.width as i32 / 2),
    MESSAGE_FRAME_BORDER.top_left.y + (MESSAGE_FRAME_BORDER.size.height as i32 / 2),
);

// Text frame layout constants
const TEXT_FRAME_THICKNESS: u32 = 2;

// Styles
const FRAME_BORDER_STYLE_BUILDER: PrimitiveStyleBuilder<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(MESSAGE_FRAME_THICKNESS);
const FRAME_BORDER_STYLE: PrimitiveStyle<BinaryColor> = FRAME_BORDER_STYLE_BUILDER.build();
const TEXT_FIELD_FRAME_STYLE_BUILDER: PrimitiveStyleBuilder<BinaryColor> =
    FRAME_BORDER_STYLE_BUILDER
        .fill_color(BinaryColor::Off)
        .stroke_width(TEXT_FRAME_THICKNESS)
        .stroke_alignment(StrokeAlignment::Center);
const TEXT_FIELD_FRAME_STYLE: PrimitiveStyle<BinaryColor> = TEXT_FIELD_FRAME_STYLE_BUILDER.build();
const TITLE_BOX_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::Off)
    .stroke_width(0)
    .fill_color(BinaryColor::On)
    .build();

const MESSAGE_TEXT_STYLE_BUILDER: TextStyleBuilder = TextStyleBuilder::new()
    .baseline(Baseline::Middle)
    .alignment(Alignment::Center);
const MESSAGE_TEXT_STYLE: TextStyle = MESSAGE_TEXT_STYLE_BUILDER.build();
const TITLE_TEXT_STYLE_BUILDER: TextStyleBuilder = MESSAGE_TEXT_STYLE_BUILDER;
const TITLE_TEXT_STYLE: TextStyle = TITLE_TEXT_STYLE_BUILDER.build();

// Fonts
const TITLE_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13_BOLD)
    .text_color(BinaryColor::Off)
    .build();
const MESSAGE_CHARACTER_STYLE_BUILDER: MonoTextStyleBuilder<'static, BinaryColor> =
    MonoTextStyleBuilder::new()
        .font(&FONT_7X14_BOLD)
        .text_color(BinaryColor::On);
const MESSAGE_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> =
    MESSAGE_CHARACTER_STYLE_BUILDER.build();

fn draw_main_screen_layout<D>(draw_target: &mut D)
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
