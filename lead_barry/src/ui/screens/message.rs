use super::common::{ScMessageImpl, TrMessage};

pub use super::common::{MessageString, MsgTitleString};

pub struct ScMessageData {
    pub title: MsgTitleString<'static>,
    pub message: MessageString<'static>,
}

impl TrMessage for ScMessageData {
    fn title(&'_ self) -> MsgTitleString<'_> {
        MsgTitleString::from_str(self.title.as_str())
    }

    fn message(&'_ self) -> MessageString<'_> {
        MessageString::from_str(self.message.as_str())
    }
}

pub type ScMessage = ScMessageImpl<ScMessageData>;
