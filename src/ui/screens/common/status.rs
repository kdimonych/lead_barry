use common::any_string::AnyString;
use core::{any::Any, usize};

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::Screen;

pub trait TrStatus {
    fn title<const SIZE: usize>(&'_ self) -> AnyString<'_, SIZE>;
    fn status<const SIZE: usize>(&'_ self) -> AnyString<'_, SIZE>;
    fn detail<const SIZE: usize>(&'_ self) -> Option<AnyString<'_, SIZE>>;
}

pub struct ScStatus<StatusT> {
    status: StatusT,
}

impl<StatusT> ScStatus<StatusT>
where
    StatusT: TrStatus,
{
    pub const fn new(status: StatusT) -> Self {
        Self { status }
    }
}

impl<StatusT> Screen for ScStatus<StatusT>
where
    StatusT: TrStatus,
{
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();
        draw_main_screen_layout(draw_target);

        let title = self.status.title::<64>();
        Text::with_text_style(
            title.as_str(),
            TITLE_TEXT_POSITION,
            TITLE_CHARACTER_STYLE,
            TITLE_TEXT_STYLE,
        )
        .draw(draw_target)
        .ok();

        let status_str = self.status.status::<64>();
        let mut status_text = Text::with_text_style(
            status_str.as_str(),
            STATUS_TEXT_POSITION,
            STATUS_CHARACTER_STYLE,
            STATUS_TEXT_STYLE,
        );

        let status_box = status_text.bounding_box();
        let mut text_box = status_box;

        if let Some(detail_str) = self.status.detail::<64>() {
            let mut detail_text = Text::with_text_style(
                detail_str.as_str(),
                STATUS_TEXT_POSITION,
                DESCRIPTION_CHARACTER_STYLE,
                DESCRIPTION_TEXT_STYLE,
            );

            const SPACE: u32 = 2;
            let detail_box = detail_text.bounding_box();
            let old_status_top_y = status_box.top_left.y;
            let old_detail_bottom_y = detail_box.top_left.y + detail_box.size.height as i32 - 1;

            let total_height = status_box.size.height + detail_box.size.height + SPACE;
            let total_top_y = STATUS_TEXT_POSITION.y - (total_height as i32 / 2);
            let total_bottom_y = total_top_y + total_height as i32 - 1;

            // Calculate the vertical offsets for the status and detail text
            let detail_y_offset = total_bottom_y - old_detail_bottom_y;
            let status_y_offset = total_top_y - old_status_top_y;

            // Apply the vertical offsets to position the texts correctly
            status_text = status_text.translate(Point::new(0, status_y_offset));
            detail_text = detail_text.translate(Point::new(0, detail_y_offset));

            //Adjust the text box to include both texts
            text_box = status_text.bounding_box();
            text_box.top_left = text_box.top_left.component_min(detail_box.top_left);
            text_box.size.width = text_box.size.width.max(detail_box.size.width);
            text_box.size.height = total_height;

            detail_text.draw(draw_target).ok();
            status_text.draw(draw_target).ok();
        } else {
            status_text.draw(draw_target).ok();
        }

        text_box = text_box.offset(2);
        if text_box.size.le(&STATUS_FRAME_BORDER
            .offset(-(STATUS_FRAME_THICKNESS as i32) - 3)
            .size)
        {
            let frame_y_mid = text_box.top_left.y + (text_box.size.height as i32) / 2;
            let text_box_right_side_x = text_box.top_left.x + text_box.size.width as i32 - 1;
            let text_box_bottom_side_y = text_box.top_left.y + text_box.size.height as i32 - 1;
            let left_corner = Point::new(text_box.top_left.x - 3, frame_y_mid);
            let right_corner = Point::new(text_box_right_side_x + 3, frame_y_mid);

            Polyline::new(&[
                Point::new(
                    STATUS_FRAME_BORDER.top_left.x + (STATUS_FRAME_THICKNESS as i32),
                    frame_y_mid,
                ),
                left_corner,
                Point::new(text_box.top_left.x, text_box.top_left.y),
                Point::new(text_box_right_side_x, text_box.top_left.y),
                right_corner,
                Point::new(
                    STATUS_FRAME_BORDER.top_left.x + (STATUS_FRAME_BORDER.size.width as i32)
                        - 1
                        - (STATUS_FRAME_THICKNESS as i32),
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

//Status frame layout constants
const STATUS_FRAME_BORDER: Rectangle = Rectangle::new(
    Point::new(SCREEN_TL.x, SCREEN_TL.y + (TITLE_HEIGHT as i32)),
    Size::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT - TITLE_HEIGHT),
);
const STATUS_FRAME_THICKNESS: u32 = 1;
const STATUS_TEXT_POSITION: Point = Point::new(
    STATUS_FRAME_BORDER.top_left.x + (STATUS_FRAME_BORDER.size.width as i32 / 2),
    STATUS_FRAME_BORDER.top_left.y + (STATUS_FRAME_BORDER.size.height as i32 / 2),
);

// Text frame layout constants
const TEXT_FRAME_THICKNESS: u32 = 2;

// Styles
const FRAME_BORDER_STYLE_BUILDER: PrimitiveStyleBuilder<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(STATUS_FRAME_THICKNESS);
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

const STATUS_TEXT_STYLE_BUILDER: TextStyleBuilder = TextStyleBuilder::new()
    .baseline(Baseline::Middle)
    .alignment(Alignment::Center);
const STATUS_TEXT_STYLE: TextStyle = STATUS_TEXT_STYLE_BUILDER.build();
const TITLE_TEXT_STYLE_BUILDER: TextStyleBuilder = STATUS_TEXT_STYLE_BUILDER;
const TITLE_TEXT_STYLE: TextStyle = TITLE_TEXT_STYLE_BUILDER.build();
const DESCRIPTION_TEXT_STYLE_BUILDER: TextStyleBuilder = STATUS_TEXT_STYLE_BUILDER;
const DESCRIPTION_TEXT_STYLE: TextStyle = DESCRIPTION_TEXT_STYLE_BUILDER.build();

// Fonts
const TITLE_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_8X13_BOLD)
    .text_color(BinaryColor::Off)
    .build();
const STATUS_CHARACTER_STYLE_BUILDER: MonoTextStyleBuilder<'static, BinaryColor> =
    MonoTextStyleBuilder::new()
        .font(&FONT_7X14_BOLD)
        .text_color(BinaryColor::On);
const STATUS_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> =
    STATUS_CHARACTER_STYLE_BUILDER.build();

const DESCRIPTION_CHARACTER_STYLE_BUILDER: MonoTextStyleBuilder<'static, BinaryColor> =
    STATUS_CHARACTER_STYLE_BUILDER;

const DESCRIPTION_CHARACTER_STYLE: MonoTextStyle<'static, BinaryColor> =
    DESCRIPTION_CHARACTER_STYLE_BUILDER.build();

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
    STATUS_FRAME_BORDER
        .into_styled(FRAME_BORDER_STYLE)
        .draw(draw_target)
        .ok();
}
