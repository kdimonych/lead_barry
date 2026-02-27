use common::any_string::AnyString;
use qrcodegen_no_heap::Mask;
use qrcodegen_no_heap::QrCode;
use qrcodegen_no_heap::QrCodeEcc;
use qrcodegen_no_heap::Version;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::*},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Polyline, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::ui::ScreenView;

const QR_CODE_VERSION: u8 = 3u8;
const QR_CODE_BUF_LENGTH: usize = Version::new(QR_CODE_VERSION).buffer_len();
pub const QR_CODE_STRING_LENGTH: usize = 47; // The maximum number of characters that can be encoded in a version 3 QR code with low error correction level is 47.

/// Type aliases for commonly used string sizes in status displays. See [`AnyString`] for more details.
pub type DmQrCodeString<'a> = AnyString<'a, QR_CODE_STRING_LENGTH>;

pub trait DataModelQrCode {
    fn qr_code<'b>(&'b self) -> &'b DmQrCodeString<'b>;
}

impl<'a> DataModelQrCode for DmQrCodeString<'a> {
    fn qr_code<'b>(&'b self) -> &'b DmQrCodeString<'a> {
        self
    }
}

pub type SvQrCode = SvQrCodeImpl<DmQrCodeString<'static>>;

impl<'a> From<DmQrCodeString<'a>> for SvQrCodeImpl<DmQrCodeString<'a>> {
    fn from(value: DmQrCodeString<'a>) -> Self {
        SvQrCodeImpl::<DmQrCodeString<'a>>::new(value)
    }
}

pub struct SvQrCodeImpl<DataModelT> {
    qr_code_model: DataModelT,
}

impl<DataModelT> SvQrCodeImpl<DataModelT> {
    pub const fn new(qr_code_model: DataModelT) -> Self
    where
        DataModelT: DataModelQrCode,
    {
        Self { qr_code_model }
    }
}

impl<DataModelT> ScreenView for SvQrCodeImpl<DataModelT>
where
    DataModelT: DataModelQrCode,
{
    fn enter<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        // Clear the display
        draw_target.clear(BinaryColor::Off).ok();
        let qr_string = self.qr_code_model.qr_code();

        let mut tempbuffer = [0u8; QR_CODE_BUF_LENGTH];
        let mut outbuffer = [0u8; QR_CODE_BUF_LENGTH];

        if let Some(qr_code) = QrCode::encode_text(
            qr_string.as_str(),
            &mut tempbuffer,
            &mut outbuffer,
            QrCodeEcc::Low,
            Version::new(QR_CODE_VERSION),
            Version::new(QR_CODE_VERSION),
            Some(Mask::new(0)),
            true,
        )
        .ok()
        {
            let module_size = core::cmp::min(
                SCREEN_WIDTH / qr_code.size() as u32,
                SCREEN_HEIGHT / qr_code.size() as u32,
            );
            let x_offset = (SCREEN_WIDTH - (qr_code.size() as u32 * module_size)) / 2;
            // Keep the QR code vertically centered, but position it towards the bottom of the screen to
            // leave space for any potential text above it. The y_offset is calculated to position the QR
            // code such that its bottom edge is a few pixels above the bottom edge of the screen.
            let y_offset = SCREEN_HEIGHT - (qr_code.size() as u32 * module_size);

            for y in 0..qr_code.size() {
                for x in 0..qr_code.size() {
                    let color = if qr_code.get_module(x, y) {
                        BinaryColor::On
                    } else {
                        BinaryColor::Off
                    };
                    let rect = Rectangle::new(
                        Point::new(
                            x_offset as i32 + (x as i32 * module_size as i32),
                            y_offset as i32 + (y as i32 * module_size as i32),
                        ),
                        Size::new(module_size, module_size),
                    );
                    rect.into_styled(PrimitiveStyle::with_fill(color))
                        .draw(draw_target)
                        .ok();
                }
            }
        }
    }
}

/* Constants */
// Screen constants
const SCREEN_TL: Point = Point::new(0, 0);
const SCREEN_BR: Point = Point::new(127, 63);
const SCREEN_WIDTH: u32 = (SCREEN_BR.x - SCREEN_TL.x + 1) as u32;
const SCREEN_HEIGHT: u32 = (SCREEN_BR.y - SCREEN_TL.y + 1) as u32;
