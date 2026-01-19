# TachyonFX Effects Guide

This document explains the animation effects available in [tachyonfx](https://github.com/junkdog/tachyonfx), an effects and animation library for Ratatui applications.

## Overview

TachyonFX provides 40+ unique effects that can be composed and layered to create sophisticated terminal animations. Effects operate on terminal cells **after** widgets have been rendered, modifying properties like colors, characters, or visibility.

### Core Concepts

1. **Effects are stateful** - Create once, apply every frame
2. **Effects transform rendered content** - Apply after widgets render
3. **Effects compose** - Build complex animations from simple pieces

### Basic Usage

```rust
use tachyonfx::{fx, EffectManager, Interpolation};

// Create an effect
let effect = fx::fade_to(Color::Cyan, Color::Gray, (500, Interpolation::SineIn));

// In render loop
effect.process(delta_time, buf, area);
```

---

## Color Effects

Transform colors over time for smooth transitions.

### `fade_from` / `fade_to`

Transition both foreground and background colors over time.

```rust
// Fade from black/white to current colors
fx::fade_from(Color::Black, Color::White, 500)

// Fade current colors to cyan/gray
fx::fade_to(Color::Cyan, Color::Gray, 500)
```

**Use case**: Dialog appearance/disappearance, focus transitions

### `fade_from_fg` / `fade_to_fg`

Transition only foreground colors.

```rust
// Text fades in from red
fx::fade_from_fg(Color::Red, 500)

// Text fades to green
fx::fade_to_fg(Color::Green, 500)
```

**Use case**: Text highlighting, status changes

### `hsl_shift` / `hsl_shift_fg`

Animate through HSL (Hue, Saturation, Lightness) color space for smooth color cycling.

```rust
// Shift hue by 180 degrees over 1 second
fx::hsl_shift((180.0, 0.0, 0.0), 1000)
```

**Use case**: Rainbow effects, pulsing glow, attention-grabbing animations

### `term256_colors`

Downsample colors to 256-color mode for terminal compatibility.

**Use case**: Ensuring compatibility with older terminals

---

## Text & Motion Effects

Animate text and cell positions for dynamic content.

### `coalesce` / `coalesce_from`

Text materialization - characters appear to "come together" from random symbols.

```rust
// Text materializes over 500ms
fx::coalesce(500)

// Materialize from specific character set
fx::coalesce_from("*#@", 500)
```

**Use case**: Dramatic text reveals, sci-fi terminal aesthetics

### `dissolve` / `dissolve_to`

Text dissolution - the opposite of coalesce. Characters scatter into random symbols.

```rust
// Text dissolves over 500ms
fx::dissolve(500)

// Dissolve into specific characters
fx::dissolve_to(".", 500)
```

**Use case**: Exit animations, content removal

### `evolve` / `evolve_into` / `evolve_from`

Character evolution through custom symbol sets. Characters morph through a sequence.

```rust
// Evolve through a sequence of characters
fx::evolve(&['.', 'o', 'O', '@'], 500)
```

**Use case**: Loading indicators, transformation effects

### `slide_in` / `slide_out`

Directional sliding animations - content moves in from or out to an edge.

```rust
use tachyonfx::Motion;

// Slide in from left
fx::slide_in(Motion::LeftToRight, 500)

// Slide out to bottom
fx::slide_out(Motion::UpToDown, 500)
```

**Directions available**:
- `LeftToRight` / `RightToLeft`
- `UpToDown` / `DownToUp`

**Use case**: Panel transitions, menu animations

### `sweep_in` / `sweep_out`

Color sweep transitions - a wave of color moves across the content.

```rust
// Green sweep from left
fx::sweep_in(Motion::LeftToRight, Color::Green, 500)
```

**Use case**: Selection highlighting, reveal animations

### `explode`

Particle dispersion effect - cells scatter outward from center.

```rust
fx::explode(500)
```

**Use case**: Destruction animations, dramatic exits

### `expand`

Bidirectional expansion from center - content grows outward.

```rust
fx::expand(500)
```

**Use case**: Dialog popups, notification reveals

### `stretch`

Unidirectional stretching with block characters - content stretches in one direction.

```rust
fx::stretch(Motion::LeftToRight, 500)
```

**Use case**: Progress indicators, loading bars

---

## Control Effects

Fine-tune timing and behavior of other effects.

### `parallel`

Run multiple effects simultaneously.

```rust
fx::parallel(&[
    fx::fade_from_fg(Color::Red, 500),
    fx::slide_in(Motion::LeftToRight, 800),
])
```

**Use case**: Complex multi-property animations

### `sequence`

Chain effects one after another.

```rust
fx::sequence(&[
    fx::fade_from_fg(Color::Black, 300),
    fx::coalesce(500),
])
```

**Use case**: Multi-stage animations

### `repeat` / `repeating`

Loop effects with optional limits or indefinitely.

```rust
// Repeat 3 times
fx::repeat(3, fx::fade_from_fg(Color::Red, 500))

// Repeat forever
fx::repeating(fx::hsl_shift((360.0, 0.0, 0.0), 2000))
```

**Use case**: Continuous animations, attention pulses

### `ping_pong`

Play effect forward then reverse, creating a bouncing animation.

```rust
fx::ping_pong(fx::fade_to_fg(Color::Cyan, 500))
```

**Use case**: Breathing effects, pulsing highlights

### `delay` / `sleep`

Add pauses before or during effects.

```rust
// Wait 200ms before starting
fx::delay(200, fx::fade_in(500))

// Just pause
fx::sleep(500)
```

**Use case**: Staggered animations, timed sequences

### `prolong_start` / `prolong_end`

Extend effect duration at the beginning or end.

```rust
// Hold initial state for extra 200ms
fx::prolong_start(200, fx::fade_in(500))
```

**Use case**: Emphasizing animation states

### `freeze_at`

Freeze effect at a specific transition point (0.0 - 1.0).

```rust
// Freeze at 50% completion
fx::freeze_at(0.5, fx::dissolve(500))
```

**Use case**: Partial effects, static states with effect styling

### `never_complete` / `timed_never_complete`

Run effects indefinitely (optionally with a time limit).

```rust
// Run forever
fx::never_complete(fx::hsl_shift((360.0, 0.0, 0.0), 1000))

// Run for max 10 seconds
fx::timed_never_complete(10000, effect)
```

**Use case**: Background animations, ambient effects

### `with_duration`

Override effect duration.

```rust
fx::with_duration(1000, fx::dissolve(500)) // Now takes 1000ms
```

---

## Spatial Patterns

Control how effects spread and progress across the terminal area.

### `RadialPattern`

Expand outward from a center point in a circular pattern.

```rust
fx::dissolve(800).with_pattern(RadialPattern::center())

// Custom origin
RadialPattern::new(0.25, 0.25) // Top-left quadrant
```

**Use case**: Ripple effects, explosions from a point

### `DiagonalPattern`

Sweep across diagonally from corner to corner.

```rust
fx::fade_to_fg(Color::Cyan, 1000)
    .with_pattern(DiagonalPattern::top_left_to_bottom_right())

// With softer transition edge
DiagonalPattern::top_left_to_bottom_right()
    .with_transition_width(3.0)
```

**Use case**: Wipe transitions, reveal effects

### `CheckerboardPattern`

Alternate cell-by-cell in a grid pattern.

```rust
fx::dissolve(500).with_pattern(CheckerboardPattern::new())
```

**Use case**: Pixelated transitions, retro effects

### `SweepPattern`

Linear progression in cardinal directions.

```rust
fx::fade_in(500).with_pattern(SweepPattern::left_to_right())
```

**Use case**: Simple directional reveals

### `CoalescePattern` / `DissolvePattern`

Organic, randomized reveals with noise-based distribution.

```rust
fx::coalesce(500).with_pattern(CoalescePattern::new())
```

**Use case**: Natural-looking text appearance

---

## Geometry Effects

Transform positions and layout.

### `translate`

Move content by an offset.

```rust
fx::translate((5, 2), 500) // Move 5 right, 2 down
```

**Use case**: Shake effects, position animations

### `resize_area`

Scale effect bounds.

**Use case**: Growing/shrinking regions

### `translate_buf`

Copy and move buffer content to another location.

**Use case**: Duplicating content, screen transitions

---

## Cell Filtering

Apply effects selectively to specific cells.

```rust
// Only affect red text
fx::dissolve(500).with_filter(CellFilter::FgColor(Color::Red))

// Only affect text (not empty cells)
fx::fade_in(500).with_filter(CellFilter::Text)

// Combine filters
let filter = CellFilter::AllOf(vec![
    CellFilter::Outer(Margin::new(1, 1)),
    CellFilter::Text,
]);
```

**Available filters**:
- `FgColor(Color)` - Match foreground color
- `BgColor(Color)` - Match background color
- `Text` - Non-empty cells
- `Inner(Margin)` / `Outer(Margin)` - Positional
- `AllOf(Vec)` / `AnyOf(Vec)` - Logical combinations
- `Not(Box)` - Negation

---

## Interpolation (Easing)

Control the acceleration curve of animations.

```rust
use tachyonfx::Interpolation;

fx::fade_in((500, Interpolation::QuadOut))
```

**Available interpolations**:

| Easing | Description |
|--------|-------------|
| `Linear` | Constant speed |
| `QuadIn/Out/InOut` | Quadratic acceleration |
| `CubicIn/Out/InOut` | Cubic acceleration |
| `QuartIn/Out/InOut` | Quartic acceleration |
| `QuintIn/Out/InOut` | Quintic acceleration |
| `SineIn/Out/InOut` | Sinusoidal acceleration |
| `ExpoIn/Out/InOut` | Exponential acceleration |
| `CircIn/Out/InOut` | Circular acceleration |
| `ElasticIn/Out/InOut` | Elastic bounce |
| `BackIn/Out/InOut` | Overshoot |
| `BounceIn/Out/InOut` | Bouncing |

**Naming convention**:
- `In` - Start slow, end fast
- `Out` - Start fast, end slow
- `InOut` - Slow at both ends

---

## Integration with opencode-helix

To add tachyonfx to the project:

```toml
[dependencies]
tachyonfx = "0.22"
```

### Recommended Effects for Dialogs

| Effect | When to Use |
|--------|-------------|
| `fade_from` + `expand` | Dialog open |
| `fade_to` + `dissolve` | Dialog close |
| `slide_in` | Menu appearance |
| `coalesce` | Text reveal |
| `ping_pong` + `hsl_shift_fg` | Focus indicator |

### Example: Animated Dialog

```rust
use tachyonfx::{fx, EffectManager, Interpolation};

// On dialog open
let open_effect = fx::parallel(&[
    fx::fade_from(Color::Black, Color::Reset, (300, Interpolation::QuadOut)),
    fx::expand((300, Interpolation::QuadOut)),
]);

// On dialog close  
let close_effect = fx::parallel(&[
    fx::fade_to(Color::Black, Color::Reset, (200, Interpolation::QuadIn)),
    fx::dissolve((200, Interpolation::QuadIn)),
]);
```

---

## Resources

- [GitHub Repository](https://github.com/junkdog/tachyonfx)
- [API Documentation](https://docs.rs/tachyonfx)
- [Interactive Editor (TachyonFX FTL)](https://junkdog.github.io/tachyonfx-ftl/)
- [Example: exabind](https://junkdog.github.io/exabind/) - Try effects in browser
