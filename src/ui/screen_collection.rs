use crate::ui::Screen;
use crate::ui::screen_animation::AnimationScreen;
use crate::ui::screen_vip::VIPScreen;
use crate::ui::screen_welcome::WelcomeScreen;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::DrawTarget;
#[derive(Clone)]
pub enum ScreenCollection {
    Welcome(WelcomeScreen),
    Animation(AnimationScreen),
    VIP(VIPScreen),
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
            ScreenCollection::Empty => (),
        }
    }
}
