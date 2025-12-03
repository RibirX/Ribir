# Guide

This guide introduces the fundamental concepts and usage patterns of Ribir, a non-intrusive GUI framework for Rust that allows you to build multi-platform applications from a single codebase.

## Core Concepts

### Declarative UI
Ribir uses a declarative approach to UI development, where you describe what your UI should look like based on its current state. The framework handles rendering and updates automatically when state changes.

### Widgets and Composition
Everything in Ribir is built from widgets. You compose complex UIs by combining simple widgets together using the `fn_widget!` macro with the `@` syntax:

```rust no_run
use ribir::prelude::*;
fn_widget! {
  @Container {
    size: Size::new(100., 100.),
    background: Color::RED,
    @Text { text: "Hello, Ribir!" }
  }
};
```

### State Management
Ribir provides reactive state management through `Stateful` objects. You can read state with `$read`, write to state with `$write`, and create reactive bindings with `pipe!` and `watch!`:

```rust no_run
use ribir::prelude::*;
let counter = Stateful::new(0);
fn_widget! {
  @Column {
    @Text { text: pipe!($read(counter).to_string()) }
    @Button { 
      on_tap: move |_| *$write(counter) += 1,
      @{ "Increment" }
    }
  }
};
```

### Layout System
Ribir's layout system uses constraints that flow down the widget tree, with size information flowing back up. The `clamp` attribute allows you to control how widgets size themselves within their available space.

### Data Sharing and Events
Use `Provider` to share data top-down through the widget tree, and `Custom Events` to bubble events from child widget up to parent widget.

## Getting Started

To start building with Ribir, explore these core concepts in detail:
- **[Declarative UI](./core_concepts/declarative_ui.md)**: Understanding the DSL syntax and widget composition
- **[Built-in Attributes & FatObj](./core_concepts/built_in_attributes_and_fat_obj.md)**: Using built-in functionality
- **[State Management](./core_concepts/state_management.md)**: Managing your application's state
- **[Data Sharing & Events](./core_concepts/data_sharing_and_events.md)**: Communication between components
- **[Widget System](./core_concepts/widgets_composition.md)**: Understanding the architecture
- **[Layout System](./core_concepts/layout.md)**: Controlling how your UI is arranged

Once you're comfortable with these basics, move on to advanced topics to extend your knowledge:
- **[Custom Widgets](./advanced/custom_widgets.md)**: Building reusable components
- **[Animations](./advanced/animations.md)**: Adding motion to your UI
- **[Theming](./advanced/theming.md)**: Customizing appearance and style
- **[Widgets](./advanced/widgets.md)**: Widget components in detail