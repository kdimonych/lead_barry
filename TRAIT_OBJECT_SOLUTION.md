# UI Trait Object Safety Solution

## Problem

The original `Screen` trait was not "dyn compatible" (object-safe) due to using a generic method:

```rust
pub trait Screen {
    fn draw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>;
}
```

When trying to use `&mut dyn Screen`, Rust complained because:

1. The `draw` method has a generic parameter `D`
2. Trait objects require all methods to be monomorphic (no generics)
3. This prevents dynamic dispatch from working

## Solution Approaches

### 1. Generic UI Struct (Current Implementation)

Instead of using trait objects, we made the `Ui` struct generic over the screen type:

```rust
pub struct Ui<ActiveScreen, I2cDevice, DisplaySize>
where
    DisplaySize: ssd1306::size::DisplaySizeAsync,
    ActiveScreen: Screen,
{
    display: Ssd1306Async<...>,
    active_screen: ActiveScreen,
}
```

**Pros:**

- Zero-cost abstraction (compile-time polymorphism)
- Type safety guaranteed at compile time
- No runtime overhead
- Works with embedded-graphics trait requirements

**Cons:**

- Cannot switch screen types at runtime without creating new UI instances
- Each screen type creates a different UI type
- More complex type signatures

### 2. Enum-Based Screen Switching (Alternative)

For runtime screen switching, we can use an enum:

```rust
pub enum ScreenType {
    Welcome(WelcomeScreen),
    Animation(AnimationScreen),
    Empty(EmptyScreen),
}

impl Screen for ScreenType {
    fn draw<D>(&mut self, draw_target: &mut D)
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        match self {
            ScreenType::Welcome(screen) => screen.draw(draw_target),
            ScreenType::Animation(screen) => screen.draw(draw_target),
            ScreenType::Empty(screen) => screen.draw(draw_target),
        }
    }
}
```

**Pros:**

- Runtime screen switching
- Still zero-cost (enum dispatch is fast)
- Type-safe
- Works with embedded-graphics

**Cons:**

- Must enumerate all possible screen types at compile time
- Larger memory footprint (size of largest variant)

## Why This Problem Occurs

The root issue is that `embedded_graphics::DrawTarget` is itself a trait with associated types:

```rust
pub trait DrawTarget {
    type Color: PixelColor;
    type Error: Debug;
    // ... methods
}
```

When you have a generic method like `fn draw<D: DrawTarget>`, you're asking the trait object system to handle an infinite number of possible types `D`, which is impossible for dynamic dispatch.

## Best Practices for Embedded UI

1. **Use compile-time polymorphism** when possible (generics)
2. **Use enums for runtime switching** when you know all screen types
3. **Avoid trait objects** for complex traits like `DrawTarget`
4. **Keep screen state minimal** to reduce memory usage
5. **Use static allocation** instead of heap allocation

## Example Usage

```rust
// Create UI with specific screen type
let welcome_screen = WelcomeScreen::new();
let mut ui = Ui {
    display: create_display(),
    active_screen: welcome_screen,
};

// Or use enum for switching
let mut screen = ScreenType::Welcome(WelcomeScreen::new());
screen.switch_to_animation(); // Runtime switching
```

## Memory Considerations

In embedded systems:

- Generic approach: Each screen type is a separate instantiation
- Enum approach: Memory = size of largest screen variant
- Trait object approach: Would need heap allocation (not suitable for no_std)

This solution provides both compile-time safety and runtime flexibility while maintaining embedded-friendly characteristics.
