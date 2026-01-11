---
sidebar_position: 1
---

# Custom Widgets

Custom widgets are the building blocks of any Ribir application. They allow you to encapsulate complex UI functionality, state, and behavior into reusable components that can be composed together to form larger applications.

## Understanding Widget Types

In Ribir, there are two main categories of custom widgets you can create:

1. **Compose Widgets**: High-level widgets that build UI by combining other widgets using the `fn_widget!` macro
2. **Render Widgets**: Low-level widgets that handle layout and painting directly

## Creating Compose Widgets

The most common type of custom widget is a `Compose` widget. These widgets don't draw anything themselves; instead, they compose other existing widgets to create something new.

### Basic Structure

To create a custom `Compose` widget, you need to:

1. Define a struct with `#[derive(Declare)]`
2. Implement the `Compose` trait
3. Use the `fn_widget!` macro in the `compose` method

```rust no_run
use ribir::prelude::*;

#[declare]
pub struct DocWelcomeCard;

impl Compose for DocWelcomeCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Column {
                @Text { text: "Welcome!" }
                @Button { @Text { text: "Click me" } }
            }
        }.into_widget()
    }
}
```

The `compose` method must return a `Widget`, so we need to call `.into_widget()` on the `fn_widget!` result.

### Using the Custom Widget

Once you've defined your custom widget, you can use it just like any built-in widget in the `fn_widget!` DSL:

```rust ignore
use ribir::prelude::*;

fn main() {
    App::run(fn_widget! {
        @DocWelcomeCard {}
    });
}
```

### Adding Properties to Custom Widgets

You can add properties to your custom widget by adding fields to your struct. These fields can be initialized in the DSL using the same syntax as built-in widgets:

```rust no_run
use ribir::prelude::*;

#[declare]
pub struct DocUserCard {
    name: String,
    email: String,
    #[declare(default)]
    is_online: bool,
}

impl Compose for DocUserCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Container {
                padding: EdgeInsets::all(16.),
                border: Border::all(BorderSide::new(1., Color::GRAY.into())),
                @Column {
                    @Text {
                        text: pipe!($read(this).name.clone()),
                    }
                    @Text {
                        text: pipe!($read(this).email.clone()),
                    }
                    @Row {
                        @Text {
                            text: pipe!($read(this).is_online).map(|v| if v { "Online" } else { "Offline" }),
                            foreground: pipe!($read(this).is_online).map(|v| if v { Color::GREEN } else { Color::GRAY }),
                        }
                        @Container {
                            size: Size::new(10., 10.),
                            margin: EdgeInsets::horizontal(8.),
                            background: pipe!($read(this).is_online).map(|v| if v { Color::GREEN } else { Color::GRAY }),
                            radius: Radius::all(5.),
                        }
                    }
                }
            }
        }.into_widget()
    }
}

// Usage:
fn example() -> Widget<'static> {
    fn_widget! {
        @DocUserCard {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            is_online: true,
        }
    }.into_widget()
}
```

> **Note:** When using `#[derive(Declare)]`, fields are **mandatory** by default. Any field that does not have the `#[declare(default)]` or `#[declare(skip)]` attribute must be provided when declaring the widget. Fields with `#[declare(default)]` are optional, while `#[declare(skip)]` excludes the field from the builder entirely.

## Creating ComposeChild Widgets

Some widgets are designed to wrap or modify a single child widget. These implement the `ComposeChild` trait instead of `Compose`.

```rust no_run
use ribir::prelude::*;

#[derive(Declare, Clone)]
pub struct DocCardDecorator {
    #[declare(default)]
    elevation: f32,
}

impl<'a> ComposeChild<'a> for DocCardDecorator {
    type Child = Widget<'a>;

    fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
        fn_widget! {
            @Container {
                padding: EdgeInsets::all(16.),
                background: Color::WHITE,
                // Add shadow based on elevation
                transform: pipe!($read(this).elevation).map(|e| {
                    Transform::scale(1. - e * 0.01, 1. - e * 0.01)
                }),
                @ { child }
            }
        }.into_widget()
    }
}

// Usage:
fn example() -> Widget<'static> {
    fn_widget! {
        @DocCardDecorator {
            elevation: 4.,
            @Text { text: "This text is inside a card" }
        }
    }.into_widget()
}
```

## Understanding the Child System

Ribir has a strict type system for parent-child relationships that ensures type safety at compile time:

- **SingleChild**: Widgets that accept exactly one child (like `Padding`, `Container`)
- **MultiChild**: Widgets that accept multiple children (like `Row`, `Column`)

The `#[derive(Declare)]` macro can automatically implement the appropriate child system trait based on your struct's fields:

```rust ignore
use ribir::prelude::*;

// For example, our Container Widget can accept a single child:
#[derive(Declare, SingleChild)]
pub struct Container {
    pub size: Size,
}

impl Render for Container {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        let size = clamp.clamp(self.size);
        ctx.perform_single_child_layout(BoxClamp::max_size(size));
        size
    }

    #[inline]
    fn size_affected_by_child(&self) -> bool { false }
}

// Usage:
fn example() -> Widget<'static> {
    fn_widget! {
        @Container {
            size: Size::new(100., 100.),
            @Text { text: "Hello" } // Can accept a Child Widget
        }
    }.into_widget()
}
```

### Template-Based Child Composition

Templates provide compile-time type safety for widget composition. The `#[derive(Template)]` macro enables **automatic type inference**, allowing you to write children without explicit type constructors or field names.

#### Automatic Type Inference

When using Templates, Ribir automatically infers:
- **Enum variants** based on child type (via `RFrom` trait)
- **Struct fields** based on child type (via `ComposeWithChild` trait)

This means you can write `@{ child }` and Ribir will automatically determine where it belongs in your template structure.

#### Enum Templates: Variant Inference

Enum templates automatically convert children to the appropriate variant:

```rust ignore
use ribir::prelude::*;

// Define an enum template with different variant types
#[derive(Template)]
enum ContentType {
    Text(CowArc<str>),
    Number(i32),
}

#[declare]
struct MyWidget;

impl<'a> ComposeChild<'a> for MyWidget {
    type Child = ContentType;

    fn compose_child(_: impl StateWriter<Value = Self>, _child: Self::Child) -> Widget<'a> {
        Void {}.into_widget()
    }
}

// Usage - automatic variant inference:
let text_widget = fn_widget! {
    @MyWidget {
        @{ "Hello" }  // Automatically becomes ContentType::Text
    }
};

let number_widget = fn_widget! {
    @MyWidget {
        @{ 42 }  // Automatically becomes ContentType::Number
    }
};
```

The `#[derive(Template)]` macro generates `RFrom` implementations for each variant, enabling automatic conversion based on the child's type.

#### Struct Templates: Field Inference

Struct templates automatically match children to fields by type, **regardless of declaration order**:

```rust ignore
use ribir::prelude::*;

// Define custom types for demonstration
struct TypeA;
struct TypeB;
struct TypeC;

#[derive(Template)]
struct StructTemplate {
    a: TypeA,
    b: Option<TypeB>,
    c: Option<TypeC>,
}

#[declare]
struct MyContainer;

impl ComposeChild<'static> for MyContainer {
    type Child = StructTemplate;

    fn compose_child(_: impl StateWriter<Value = Self>, _child: Self::Child) -> Widget<'static> {
        Void {}.into_widget()
    }
}

// Usage - order-independent field matching:
let widget = fn_widget! {
    @MyContainer {
        @{ TypeC }  // Matched to 'c' field by type
        @{ TypeA }  // Matched to 'a' field by type
        @{ TypeB }  // Matched to 'b' field by type
    }
};

// Optional fields can be omitted:
let minimal = fn_widget! {
    @MyContainer {
        @{ TypeA }  // Only required field
    }
};
```

The macro generates `ComposeWithChild` implementations with type-specific markers for each field, enabling automatic field assignment.

#### Real-World Example: List Widget

The `List` widget demonstrates practical template usage:

```rust ignore
use ribir::prelude::*;

// Simplified from widgets/src/list.rs
#[derive(Template)]
pub enum ListChild<'c> {
    StandardItem(PairOf<'c, ListItem>),
    CustomItem(PairOf<'c, ListCustomItem>),
    Divider(FatObj<Stateful<Divider>>),
}

impl<'c> ComposeChild<'c> for List {
    type Child = Vec<ListChild<'c>>;
    // ...
}

// Usage - automatic variant inference:
let list = fn_widget! {
    @List {
        @ListItem { /* ... */ }      // Automatically becomes ListChild::StandardItem
        @ListCustomItem { /* ... */ } // Automatically becomes ListChild::CustomItem
        @Divider {}                   // Automatically becomes ListChild::Divider
    }
};
```

#### When Explicit Syntax Is Required

Automatic inference works when types are unique. Use explicit syntax when:

1. **Multiple fields have the same type** - use `#[template(field)]` attribute:
```rust ignore
#[derive(Template)]
struct TwoTexts {
    #[template(field)]
    first: CowArc<str>,
    #[template(field)]
    second: CowArc<str>,
}

// Must use explicit field assignment:
let widget = fn_widget! {
    @MyWidget {
        @TwoTexts {
            first: "First text",
            second: "Second text",
        }
    }
};
```

2. **Non-widget template fields** (use `#[template(field)]` attribute):
```rust ignore
struct TypeA;

#[derive(Template)]
struct ConfigTemplate {
    #[template(field = 5usize)]  // Default value
    count: usize,
    #[template(field)]           // Required field
    name: CowArc<str>,
    item: TypeA,                 // Child field (auto-matched by type)
}

// Can override default or omit to use default:
let widget = fn_widget! {
    @MyWidget {
        @ConfigTemplate {
            count: 10usize,  // Override default
            name: "test",    // Required field
            @{ TypeA }       // Child matched by type
        }
    }
};

// Using default value:
let widget2 = fn_widget! {
    @MyWidget {
        @ConfigTemplate {
            name: "test",  // count uses default value of 5
            @{ TypeA }
        }
    }
};
```

This template system ensures type-safe widget composition while minimizing boilerplate through intelligent type inference.

## Advanced: Creating Render Widgets

For widgets that need to handle their own layout and painting (like drawing custom shapes or complex interactions), you implement the `Render` trait:

```rust ignore
use ribir::prelude::*;

// This is a simple example - more complex Render widgets would
// implement custom layout and painting logic
#[declare]
pub struct DocCustomShape {
    #[declare(default)]
    color: Color,
    #[declare(default)]
    size: Size,
}

impl Render for DocCustomShape {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // Return the size based on constraints and our desired size
        clamp.clamp(self.size)
    }

    fn paint(&self, ctx: &mut PaintingCtx) {
        // Custom painting logic
        let rect = Rect::from_size(ctx.box_rect().unwrap().size);
        ctx
          .painter()
          .rect(&rect)
          .set_fill_brush(self.color)
          .fill();
    }
}
```

## Best Practices

1. **Use `#[derive(Declare)]`**: This macro generates the builder pattern needed for your widget to work with the `@` syntax
2. **Field Requirements**: Fields are mandatory by default. Use `#[declare(default)]` for optional fields or `#[declare(skip)]` to exclude them from the builder.
3. **State encapsulation**: Keep widget state encapsulated and avoid global state
4. **Reusability**: Design widgets to be reusable and composable
5. **Performance**: Be mindful of expensive operations in `perform_layout` and `paint` methods

## Summary

Custom widgets form the foundation of any Ribir application. By understanding the difference between `Compose` and `Render` widgets, and how to properly define and use state, you can create powerful, reusable components that leverage the full power of Ribir's declarative UI system.