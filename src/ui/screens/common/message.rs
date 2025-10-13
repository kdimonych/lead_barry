use common::any_string::AnyString;
use core::str::SplitWhitespace;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::Screen;

const TITLE_LENGTH: usize = 15;
const MESSAGE_LINE_LENGTH: usize = 18;
const MESSAGE_LENGTH: usize = MESSAGE_LINE_LENGTH * 3; // Support for three lines of message

/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type MsgTitleString<'a> = AnyString<'a, TITLE_LENGTH>;
/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type MessageString<'a> = AnyString<'a, MESSAGE_LENGTH>;

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
        let lines = split_message_into_lines(message_str.as_str()).unwrap_or_else(|_| {
            let mut err_str = heapless::String::<MESSAGE_LINE_LENGTH>::new();
            err_str.push_str("Message too long").ok();
            let mut vec = heapless::Vec::<heapless::String<MESSAGE_LINE_LENGTH>, 3>::new();
            vec.push(err_str).ok();
            vec
        });

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

        text_box = text_box.offset(2);
        if text_box.size.le(&MESSAGE_FRAME_BORDER
            .offset(-(MESSAGE_FRAME_THICKNESS as i32) - 3)
            .size)
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

// TODO: Cover with tests
fn split_message_into_lines(
    message: &str,
) -> Result<heapless::Vec<heapless::String<MESSAGE_LINE_LENGTH>, 3>, &'static str> {
    let mut lines: heapless::Vec<heapless::String<MESSAGE_LINE_LENGTH>, 3> = heapless::Vec::new();
    let mut current_line = heapless::String::<MESSAGE_LINE_LENGTH>::new();

    let push_line = |lines: &mut heapless::Vec<heapless::String<_>, 3>,
                     current_line: &mut heapless::String<_>| {
        lines.push(current_line.clone()).ok();
        current_line.clear();
    };

    for line in message.split('\n') {
        if lines.is_full() {
            return Err("Too long message");
        }
        if current_line.len() + line.len() <= MESSAGE_LINE_LENGTH {
            current_line.push_str(line);
        } else {
            for word in line.split_whitespace() {
                loop {
                    if lines.is_full() {
                        return Err("Too long message");
                    }
                    if current_line.len() + word.len() <= MESSAGE_LINE_LENGTH {
                        if !current_line.is_empty() {
                            current_line.push(' ');
                        }
                        current_line.push_str(word);
                        break;
                    } else if word.len() > MESSAGE_LINE_LENGTH {
                        return Err("Too long word");
                    }

                    push_line(&mut lines, &mut current_line);
                }
            }
        }
        push_line(&mut lines, &mut current_line);
    }

    Ok(lines)
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
