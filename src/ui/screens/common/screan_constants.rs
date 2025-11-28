use embedded_graphics::prelude::Point;

// Screen constants
pub const SCREEN_TL: Point = Point::new(0, 0);
pub const SCREEN_BR: Point = Point::new(127, 63);
pub const SCREEN_WIDTH: u32 = (SCREEN_BR.x - SCREEN_TL.x + 1) as u32;
pub const SCREEN_HEIGHT: u32 = (SCREEN_BR.y - SCREEN_TL.y + 1) as u32;
pub const SCREEN_MIDDLE_X: i32 = SCREEN_TL.x + (SCREEN_WIDTH / 2) as i32;
