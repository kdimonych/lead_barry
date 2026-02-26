use embedded_graphics::{draw_target::DrawTarget, pixelcolor::BinaryColor};

pub trait Screen {
    fn redraw<D>(&mut self, _draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>;

    fn enter<D>(&mut self, _draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
    }

    fn exit<D>(&mut self, _draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
    }
}
