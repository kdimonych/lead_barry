mod ip_satus;
mod vcp;
mod welcome;
mod wifi_ap;
mod wifi_status;

pub use crate::ui::screens::ip_satus::{IpState, ScIpStatus};
pub use crate::ui::screens::vcp::{BaseUnits, ScVcp};
pub use crate::ui::screens::welcome::ScWelcome;
pub use crate::ui::screens::wifi_ap::ScWifiAp;
pub use crate::ui::screens::wifi_status::{ScWifiStats, State};

pub use crate::ui::screen::Screen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
pub enum ScCollection {
    Welcome(ScWelcome),
    Vcp(ScVcp),
    WiFiStatus(ScWifiStats),
    WiFiAp(ScWifiAp),
    IpStatus(ScIpStatus),
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
