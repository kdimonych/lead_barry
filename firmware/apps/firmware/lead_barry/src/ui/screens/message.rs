use super::common::{DataModelMessage, SvMessageImpl};

pub use super::common::{MessageString, MsgTitleString};

/// A simple data model for a message screen, containing a title and a message.
/// This struct implements the `DataModelMessage` trait, which allows it to be used with
/// the `SvMessageImpl` screen implementation.
pub struct DmMessage {
    pub title: MsgTitleString<'static>,
    pub message: MessageString<'static>,
}

impl DataModelMessage for DmMessage {
    fn title<'b>(&'b self) -> &'b MsgTitleString<'b> {
        &self.title
    }

    fn message<'b>(&'b self) -> &'b MessageString<'b> {
        &self.message
    }
}

/// A screen that displays a message with a title. The content of the message is
/// provided by the `DmMessage` data model.
pub type SvMessage = SvMessageImpl<DmMessage>;
