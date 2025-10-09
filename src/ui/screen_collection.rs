use crate::ui::Screen;
use crate::ui::screen_animation::AnimationScreen;
use crate::ui::screen_vcp::VIPScreen;
use crate::ui::screen_welcome::WelcomeScreen;
use crate::ui::screen_wifi_status::WifiStatsScreen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
#[derive(Clone)]
pub enum ScreenCollection {
    Welcome(WelcomeScreen),
    Animation(AnimationScreen),
    VIP(VIPScreen),
    WiFiStatus(WifiStatsScreen),
    Empty,
}

impl Screen for ScreenCollection {
    fn redraw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScreenCollection::Welcome(screen) => screen.redraw(draw_target),
            ScreenCollection::Animation(screen) => screen.redraw(draw_target),
            ScreenCollection::VIP(screen) => screen.redraw(draw_target),
            ScreenCollection::WiFiStatus(screen) => screen.redraw(draw_target),
            ScreenCollection::Empty => (),
        }
    }
}
