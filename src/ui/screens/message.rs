use super::common::{ScMessageImpl, TrMessage};

pub use super::common::{MessageString, MsgTitleString};

pub struct ScMessageData {
    pub title: MsgTitleString<'static>,
    pub message: MessageString<'static>,
}

impl TrMessage for ScMessageData {
    fn title<'a>(&'a self) -> MsgTitleString {
        MsgTitleString::from_str(self.title.as_str())
    }

    fn message<'a>(&'a self) -> MessageString {
        MessageString::from_str(self.message.as_str())
    }
}

pub type ScMessage = ScMessageImpl<ScMessageData>;
