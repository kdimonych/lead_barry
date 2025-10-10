mod ip_satus;
mod vcp;
mod welcome;
mod wifi_ap;
mod wifi_status;

pub use crate::ui::screens::ip_satus::{ScvIpState, ScvIpStatus};
pub use crate::ui::screens::vcp::{ScVcp, ScvBaseUnits};
pub use crate::ui::screens::welcome::ScWelcome;
pub use crate::ui::screens::wifi_ap::ScWifiAp;
pub use crate::ui::screens::wifi_status::{ScWifiStats, ScvState};

pub use crate::ui::screen::Screen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;

/// # Naming Conventions
///
/// This module follows specific naming conventions:
///
/// | Prefix | Description | Example |
/// |--------|-------------|---------|
/// | `Sc`   | Screen type | [`ScWelcome`] |
/// | `Scv`  | Screen variable type | [`ScvState`] |
///
/// Screen types follow the pattern: `Sc` + ScreenName
const _NAMING_CONVENTION_DOC: () = ();

///
/// Collection of all screens
///
/// See [`_NAMING_CONVENTION_DOC`] for naming conventions used in this module.
pub enum ScCollection {
    Welcome(ScWelcome),
    Vcp(ScVcp),
    WiFiStatus(ScWifiStats),
    WiFiAp(ScWifiAp),
    IpStatus(ScvIpStatus),
    Empty,
}

impl Screen for ScCollection {
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
            ScCollection::Empty => (),
        }
    }
}
