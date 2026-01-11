---
sidebar_position: 2
---

# Theming

Ribir provides a powerful and flexible theming system designed to help you build consistent and beautiful UIs. The color system is heavily inspired by Material Design principles but is fully customizable to fit any design language.

At its core, a `Theme` in Ribir is a collection of:
- **Palette**: A comprehensive color system.
- **Typography**: A set of semantic text styles.
- **Classes**: A powerful mechanism to separate style logic from widget structure.

The current theme is always available in the build context. You can access it using `Theme::of(ctx)`.

## 1. Palette (Colors)

The `Palette` struct defines the color scheme for your application. It supports both light and dark modes via the `Brightness` enum.

Ribir's palette uses semantic naming (like `primary`, `secondary`, `surface`, `error`) rather than descriptive naming (like `red`, `blue`). This ensures that your UI adapts correctly when the theme changes (e.g., switching from light to dark mode).

### Using Colors

```rust no_run
use ribir::prelude::*;

fn example() -> Widget<'static> {
    fn_widget! {
        let palette = Palette::of(BuildCtx::get());
        @Container {
            size: Size::new(100., 100.),
            background: palette.primary(), // Accessing the primary color
        }
    }.into_widget()
}
```

## 2. Typography (Text Styles)

Ribir's typography system organizes text styles into semantic categories like `Display`, `Headline`, `Title`, `Label`, and `Body`. Each category has `Large`, `Medium`, and `Small` variations.

This structure allows you to define a consistent typographic hierarchy across your application.

### Accessing Text Styles

You can access the typography theme using `TypographyTheme::of(ctx)`.

```rust no_run
use ribir::prelude::*;

fn text_style_example() -> Widget<'static> {
    fn_widget! {
        let typography = TypographyTheme::of(BuildCtx::get());

        @Column {
            @Text {
                text: "Main Title",
                text_style: typography.display_large.text.clone(),
            }
            @Text {
                text: "Subtitle",
                text_style: typography.title_medium.text.clone(),
            }
            @Text {
                text: "Body text goes here...",
                text_style: typography.body_medium.text.clone(),
            }
        }
    }.into_widget()
}
```

## 3. Classes (Styles)

One of Ribir's most powerful features is its `Class` system. A `Class` in Ribir is not just a collection of properties (like CSS classes); it is a **function that transforms a widget**.

This allows a class to:
- Set properties (e.g., color, padding).
- Wrap the widget (e.g., add a border or background container, add new elements, etc.).
- Add behavior (e.g., event listeners).

### Using Classes
#### Step 1: Define Class Names

Use the `class_names!` macro to define globally unique keys for your widget's styles.

```rust ignore
use ribir::prelude::*;

class_names! {
    /// The default class for MyCard
    MY_CARD,
    /// The class for MyCard's title
    MY_CARD_TITLE
}
```

#### Step 2: Use Classes in Your Widget

In your widget's `compose` method, use these class names to apply styles from the current theme.

```rust ignore
#[declare]
pub struct MyCard;

impl Compose for MyCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Stack {
                // Apply the main card style from the theme
                class: MY_CARD,
                @Text {
                    text: "Card Title",
                    // Apply the title style from the theme
                    class: MY_CARD_TITLE,
                }
            }
        }.into_widget()
    }
}
```

#### Step 3: Provide Styles

We can provide styles for MyCard through providers.

```rust ignore
use ribir::prelude::*;

fn main() {
    providers! {
      // Provide the styles on the provider
      providers: [
        Class::provider(MY_CARD, move |w| {
          fn_widget! {
            @Container {
              size: Size::new(100., 100.),
              radius: Radius::all(12.),
              clamp: BoxClamp::fixed_size(Size::splat(48.)),
              @ { w }
            }
          }.into_widget()
        }),
        Class::provider(MY_CARD_TITLE, style_class!{
          line_height: 24.,
          foreground: Color::RED, // Set the text color
        }),
      ],
      @MyCard {  }
    }.into_widget()
}
```

**Note**: For styles that add built-in attributes, Ribir provides the `style_class!` macro to quickly generate styles.

### Classes in the Theme

In the theme, you can provide some default styles for components throughout the application. For example, the `themes/material` provide Material classes for the Widgets component library that make the Widgets component library run like Material Design.
```rust ignore
use ribir_core::prelude::Classes;
//...

pub fn theme() -> Classes {
  let mut classes = Classes::default();
  
  buttons_cls::init(&mut classes); // Button styles
  badge_cls::init(&mut classes); // Badge styles
  // ..

  Theme {
    classes,
    //...
  }
}
```


## 4. Icons

Icons in Ribir are managed separately from themes using the global `svg_registry` module. This provides a centralized system for registering and retrieving SVG icons by name across your application.

### Managing Icons with SVG Registry

```rust ignore
use ribir::prelude::*;

fn register_icons() {
    // Register an SVG with a name
    let my_svg = Svg::parse_from_bytes(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">...</svg>"#.as_bytes(),
        true,  // inherit_fill
        false  // inherit_stroke
    ).unwrap();

    svg_registry::register("my_icon", my_svg);
}

fn icon_example() -> Widget<'static> {
    fn_widget! {
        // Use the registered icon with fallback to default if not found
        @Icon { @ { svg_registry::get_or_default("my_icon") } }
    }.into_widget()
}
```

The SVG registry provides functions for:
- `svg_registry::register(name, svg)`: Register an SVG with a specific name
- `svg_registry::get(name)`: Retrieve a named SVG (returns Option)
- `svg_registry::get_or_default(name)`: Retrieve a named SVG or fallback to default

