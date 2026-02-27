#![allow(dead_code)]
#![allow(unused_imports)]

mod common;
mod ip_satus;
mod message;
mod qr_code;
mod vcp;
mod welcome;
mod wifi_ap;
mod wifi_status;

pub use ip_satus::{DmIpData, IpTitleString, ScvIpState, SvIpStatus};
pub use message::{DmMessage, MessageString, MsgTitleString, SvMessage};
pub use qr_code::{DataModelQrCode, DmQrCodeString, SvQrCode, SvQrCodeImpl};
pub use vcp::{DmVcp, DmVcpBaseUnits, DmVcpTitle, SvVcp};
pub use welcome::SvWelcome;
pub use wifi_ap::{DmWifiAp, DmWifiApClientInfo, DmWifiApCredentials, SvWifiAp};
pub use wifi_status::{DmWifiStatus, DmWifiStatusState, SvWifiStatus};

pub use crate::ui::screen_view::ScreenView;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;

/// # Naming Conventions
///
/// This module follows specific naming conventions:
///
/// | Prefix       | Description                  | Example              |
/// |--------------|------------------------------|----------------------|
/// | `Sv`         | Screen View implementation.  | [`SvMessage`].       |
/// | `Dm`         | Data Model type              | [`DmMessage`]        |
/// | `DataModel`  | Data Model trait             | [`DataModelMessage`] |
/// | `DataModelT` | Data Model generic parameter | [`DataModelT`]       |
///
///
/// Screen view types follow the pattern: `Sv` + ScreenViewName
///
/// Data Model types follow the pattern: `Dm` + ScreenViewName. For example, `DmMessage`
/// is a data model type for a message screen view.
///
/// The types that subordinate Data Model types follow the pattern: `Dm` + ScreenViewName +
/// SubordinateTypeName. For example, `DmWifiApData` is a data model type for a WiFi AP screen
/// view, and `DmWifiApDataClientInfo` is a subordinate type that represents client information
/// for the WiFi AP data model.
///
/// The screen view types can have data model types as generic parameters that implement
/// the necessary traits to provide data to the screen. For example, `SvMessageImpl<DataModelT>`
/// is a screen view that takes a generic parameter `DataModelT` which must implement the
/// `DataModelMessage` trait to provide the title and message data needed for the screen.
///
/// The Screen View can be generic over the data model, but the data model must implement the
/// appropriate trait to be used with that screen view. For example, `SvMessageImpl<DataModelT>`
/// can be used with any `DataModelT` that implements the `DataModelMessage` trait, allowing
/// for flexibility in the data models that can be used with the message screen view.
///
/// The name generic Screen View types follow the pattern: `Sv` + ScreenViewName + `Impl`. For
/// example, `SvMessageImpl<DataModelT>` is a generic screen view implementation for a message
/// screen that can work with any data model that implements the `DataModelMessage` trait.
///
/// Data model traits follow the pattern: `DataModel` + ScreenViewName. For example,
///  `DataModelMessage` is a trait that defines the data model for a message screen view.
const _NAMING_CONVENTION_DOC: () = ();

///
/// Collection of all screens
///
/// See [`_NAMING_CONVENTION_DOC`] for naming conventions used in this module.
pub enum ScCollection {
    Welcome(SvWelcome),
    Vcp(SvVcp),
    WiFiStatus(SvWifiStatus),
    WiFiAp(SvWifiAp),
    IpStatus(SvIpStatus),
    Message(SvMessage),
    QrCode(SvQrCode),
    Empty,
}

/// Creates a screen collection from a welcome screen view by wrapping it in the collection enum.
impl From<SvWelcome> for ScCollection {
    fn from(value: SvWelcome) -> Self {
        ScCollection::Welcome(value)
    }
}

/// Creates a screen collection from a VCP data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmVcp> for ScCollection {
    fn from(value: DmVcp) -> Self {
        ScCollection::Vcp(value.into())
    }
}

/// Creates a screen collection from a VCP screen view by wrapping it in the collection enum.
impl From<SvVcp> for ScCollection {
    fn from(value: SvVcp) -> Self {
        ScCollection::Vcp(value)
    }
}

/// Creates a screen collection from a WiFi status screen view by wrapping it in the collection enum.
impl From<SvWifiStatus> for ScCollection {
    fn from(value: SvWifiStatus) -> Self {
        ScCollection::WiFiStatus(value)
    }
}

/// Creates a screen collection from a WiFi AP data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmWifiStatus> for ScCollection {
    fn from(value: DmWifiStatus) -> Self {
        ScCollection::WiFiStatus(value.into())
    }
}

/// Creates a screen collection from a WiFi AP screen view by wrapping it in the collection enum.
impl From<SvWifiAp> for ScCollection {
    fn from(value: SvWifiAp) -> Self {
        ScCollection::WiFiAp(value)
    }
}
/// Creates a screen collection from a WiFi AP data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmWifiAp> for ScCollection {
    fn from(value: DmWifiAp) -> Self {
        ScCollection::WiFiAp(value.into())
    }
}

/// Creates a screen collection from a WiFi AP client info data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmWifiApCredentials> for ScCollection {
    fn from(value: DmWifiApCredentials) -> Self {
        ScCollection::WiFiAp(value.into())
    }
}

/// Creates a screen collection from a WiFi AP client info data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmWifiApClientInfo> for ScCollection {
    fn from(value: DmWifiApClientInfo) -> Self {
        ScCollection::WiFiAp(value.into())
    }
}

/// Creates a screen collection from an IP status screen view by wrapping it in the collection enum.
impl From<SvIpStatus> for ScCollection {
    fn from(value: SvIpStatus) -> Self {
        ScCollection::IpStatus(value)
    }
}

/// Creates a screen collection from a message screen view by wrapping it in the collection enum.
impl From<SvMessage> for ScCollection {
    fn from(value: SvMessage) -> Self {
        ScCollection::Message(value)
    }
}

/// Creates a screen collection from a message data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmMessage> for ScCollection {
    fn from(value: DmMessage) -> Self {
        ScCollection::Message(value.into())
    }
}

/// Creates a screen collection from a QR code screen view by wrapping it in the collection enum.
impl From<SvQrCode> for ScCollection {
    fn from(value: SvQrCode) -> Self {
        ScCollection::QrCode(value)
    }
}

/// Creates a screen collection from a QR code data model by converting it into the corresponding
/// screen view and wrapping it in the collection enum.
impl From<DmQrCodeString<'static>> for ScCollection {
    fn from(value: DmQrCodeString<'static>) -> Self {
        ScCollection::QrCode(value.into())
    }
}

impl ScreenView for ScCollection {
    fn enter<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScCollection::Welcome(screen) => screen.enter(draw_target),
            ScCollection::Vcp(screen) => screen.enter(draw_target),
            ScCollection::WiFiStatus(screen) => screen.enter(draw_target),
            ScCollection::WiFiAp(screen) => screen.enter(draw_target),
            ScCollection::IpStatus(screen) => screen.enter(draw_target),
            ScCollection::Message(screen) => screen.enter(draw_target),
            ScCollection::QrCode(screen) => screen.enter(draw_target),
            ScCollection::Empty => (),
        }
    }

    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScCollection::Welcome(screen) => screen.redraw(draw_target),
            ScCollection::Vcp(screen) => screen.redraw(draw_target),
            ScCollection::WiFiStatus(screen) => screen.redraw(draw_target),
            ScCollection::WiFiAp(screen) => screen.redraw(draw_target),
            ScCollection::IpStatus(screen) => screen.redraw(draw_target),
            ScCollection::Message(screen) => screen.redraw(draw_target),
            ScCollection::QrCode(screen) => screen.redraw(draw_target),
            ScCollection::Empty => (),
        }
    }

    fn exit<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScCollection::Welcome(screen) => screen.exit(draw_target),
            ScCollection::Vcp(screen) => screen.exit(draw_target),
            ScCollection::WiFiStatus(screen) => screen.exit(draw_target),
            ScCollection::WiFiAp(screen) => screen.exit(draw_target),
            ScCollection::IpStatus(screen) => screen.exit(draw_target),
            ScCollection::Message(screen) => screen.exit(draw_target),
            ScCollection::QrCode(screen) => screen.exit(draw_target),
            ScCollection::Empty => (),
        }
    }
}
