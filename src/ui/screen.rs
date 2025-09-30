use embedded_graphics::{draw_target::DrawTarget, pixelcolor::BinaryColor};

pub trait Screen {
    fn draw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>;
}
