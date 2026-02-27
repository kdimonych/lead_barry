use super::common::{ScMessageImpl, TrMessage};

pub use super::common::{MessageString, MsgTitleString};

pub struct ScMessageData {
    pub title: MsgTitleString<'static>,
    pub message: MessageString<'static>,
}

impl TrMessage for ScMessageData {
    fn title<'b>(&'b self) -> &'b MsgTitleString<'b> {
        &self.title
    }

    fn message<'b>(&'b self) -> &'b MessageString<'b> {
        &self.message
    }
}

pub type ScMessage = ScMessageImpl<ScMessageData>;
