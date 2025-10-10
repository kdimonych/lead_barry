use crate::ui::Screen;
use crate::ui::screen_ip_satus::IpStatusScreen;
use crate::ui::screen_vcp::VIPScreen;
use crate::ui::screen_welcome::WelcomeScreen;
use crate::ui::screen_wifi_ap::WifiApScreen;
use crate::ui::screen_wifi_status::WifiStatsScreen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
pub enum Collection {
    Welcome(WelcomeScreen),
    VIP(VIPScreen),
    WiFiStatus(WifiStatsScreen),
    WiFiAp(WifiApScreen),
    IpStatus(IpStatusScreen),
    Empty,
}

impl Screen for Collection {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            Collection::Welcome(screen) => screen.redraw(draw_target),
            Collection::VIP(screen) => screen.redraw(draw_target),
            Collection::WiFiStatus(screen) => screen.redraw(draw_target),
            Collection::WiFiAp(screen) => screen.redraw(draw_target),
            Collection::IpStatus(screen) => screen.redraw(draw_target),
            Collection::Empty => (),
        }
    }
}
