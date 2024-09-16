# Changelog

All notable changes to this project will be documented in this file.

Please keep one empty line before and after all headers. (This is required for `git` to produce a conflict when a release is made while a PR is open and the PR's changelog entry would go into the wrong section).

There are 5 types of changes:

- `Features` for new features.
- `Changed` for changes in existing functionality.
- `Fixed` for any bug fixes.
- `Documented` for any changes to the documentation.
- `Breaking` for the detail of any backward incompatible changes.

Please only add new entries below the [Unreleased](#unreleased---releasedate) header with the following format:

``` md
**crate or effect scope**: description of change (#pr @contributor)
```

1.  When creating a pull request, `#pr` will automatically update with the pull request number.
2.  Replace `@contributor` with your GitHub username.

<!-- next-header -->

## [@Unreleased] - @ReleaseDate

### Features

- **core**: Added `WrapRender` for a render widget that combines with its child as a single widget tree node. (#626 @M-Adoo)
- **core:: Added `StateWriter::into_render` to covert writer to reader if no other writer exist. (#626 @M-Adoo)

### Changed

- **core**: Reimplemented `HAlignWidget`, `VAlignWidget`, `RelativeAnchor`, `BoxDecoration`, `ConstrainedBox`, `IgnorePoint`, `Opacity`, `Padding`, `TransformWidget`, and `VisibilityRender` as `WrapRender`. (#626 @M-Adoo)


### Fixed

- **core**: The `SplitWriter` and `MapWriter` of the render widget may not be flagged as dirty. (#626, @M-Adoo)


## [0.4.0-alpha.9] - 2024-09-18

### Changed

- **core**: Refactor the `LayoutCtx` to eliminate the need for performing layout based on children order. (#625 @M-Adoo)

### Breaking

- **core**: The `Layouter` has been removed, so the render widget needs to adjust the APIs used accordingly. (#625, @M-Adoo)

## [0.4.0-alpha.8] - 2024-09-11

### Features

- **core**: The built-in widget `Class` has been added to enable the sharing of consistent styles across multiple elements and to allow widgets to have different actions and styles in different themes. (#624, @M-Adoo)
- **core**: The widget `ConstrainedBox` has been added as a built-in widget; now `clamp` can be used as a built-in field. (#624 @M-Adoo)
- **core**: Added `WindowFlags` to regulate the window behavior, with the option of utilizing `WindowFlags::ANIMATIONS` to toggle animations on or off. (#624 @M-Adoo)
- **theme/material**: Define the constant variables of motion. (#624, @M-Adoo)
- **dev_helper**: Refine the widget test macros. (#624, @M-Adoo)

### Changed

- **widgets**: Utilize `Class` to implement the `Scrollbar`. (#624, @M-Adoo)

### Breaking

- **widgets**: `ConstrainedBox` has been relocated to `core`. (#624, @M-Adoo)
- **widgets**: Utilize `Scrollbar` instead of both `HScrollbar`, `VScrollbar`, and `BothScrollbar`. (#624, @M-Adoo)

### Fixed

- **macros**: Declaring the variable parent with built-in fields as immutable is incorrect if its child uses it as mutable. (#623 @M-Adoo)

## [0.4.0-alpha.7] - 2024-09-04

### Fixed

- **widgets**: Flex may not decrease the gap for the second child during layout. (#622 @M-Adoo)


## [0.4.0-alpha.6] - 2024-08-21

### Features

- **core**: Support for modifying the theme at runtime. (#618 @M-Adoo)
  <img src="./static/theme-switch.gif" style="transform:scale(0.5);"/>

  The code:

  ```rust
  use ribir::prelude::*;

  let w = fn_widget! {
    @Text {
      on_tap: |e| {
        // Query the `Palette` of the application theme.
        let mut p = Palette::write_of(e);
        if p.brightness == Brightness::Light {
          p.brightness = Brightness::Dark;
        } else {
          p.brightness = Brightness::Light;
        }
      },
      text : "Click me!"
    }
  };

  App::run(w);
  ```

- **core**: Added `Provider` widget to share data between sub-tree. (#618 @M-Adoo)
  ```rust
  Provider::new(Box::new(State::value(0i32))).with_child(fn_widget! {
    @SizedBox {
      size: Size::new(1.,1.),
      on_tap: |e| {
        // Access the provider in a callback.
        let mut v = Provider::write_of::<i32>(e).unwrap();
        *v += 1;
      },
      @Text {
        text: {
          // Access the provider in any descendants
          let v = Provider::of::<Stateful<i32>>(ctx!());
          let v = v.unwrap().clone_writer();
          pipe!($v.to_string())
        }
      }
    }
  });
  ```

- **core**: Added `Overlay::of` to allow querying the overlay in event callbacks. (#618 @M-Adoo)
- **core**: Added `WidgetCtx::query`, `WidgetCtx::query_write`, `WidgetCtx::query_of_widget` and  `WidgetCtx::query_write_of_widget`. (#618 @M-Adoo)

### Breaking

- **core**: Removed `Overlay::new_with_handle` and `OverlayCloseHandle`. (#618 @M-Adoo)
- **core**: `GenWidget::gen_widget` no longer requires a `&mut BuildCtx` parameter. (#616 @M-Adoo)
- **core**: Removed `FullTheme` and `InheritTheme`, now only using `Theme`. Any part of the theme, such as `Palette`, can be directly used to overwrite its corresponding theme component. (#618 @M-Adoo)

## [0.4.0-alpha.5] - 2024-08-14

### Features

- **core**: `PartData<T>` now supports `T: ?Sized`, allowing us to separate trait objects from `State`.(#614 @M-Adoo)

### Breaking

- **core**: Removed unnecessary `Writer` since it has the same capabilities as `Stateful`. (#615 @M-Adoo)


## [0.4.0-alpha.4] - 2024-08-07

### Features

- **core**: Introduced `IntoWidget` and `IntoChild`. (@M-Adoo #612)

  The `IntoWidget` trait allows for the conversion of any widget to the type `Widget`.
  The `IntoChild` trait provides a way to convert a more general type into a child of `ComposeChild`.

### Fixed

**core**: The generation of a pipe widget from another pipe widget may potentially result in a crash. (#612, @M-Adoo)

### Changed

- **core**: Lazy build the widget tree. (#612, @M-Adoo)
- **core**: Simplify the implementation of parent composition with child widgets. (#612, @M-Adoo)
  
  Merge `SingleWithChild`, `MultiWithChild`, and `ComposeWithChild` into a single trait called WithChild.

### Breaking

- Removed `WidgetCtx::query_widget_type` and `WidgetCtx::query_type` (#618 @M-Adoo)
- Removed `ChildFrom` and `FromAnother` traits (#612 @M-Adoo)
- Removed `SingleParent` and `MultiParent` traits. (#612 @M-Adoo)
- Removed `PairChild` and `PairWithChild` traits. User can use a generic type instead. (#612 @M-Adoo)
- Removed the all builder traits such as WidgetBuilder and ComposeBuilder and so on. (#612 @M-Adoo)
- All implicit child conversions have been removed, except for conversions to Widget. (#612 @M-Adoo)


## [0.4.0-alpha.3] - 2024-06-26

## [0.4.0-alpha.2] - 2024-06-19

### Features

- **core**: Added support to query a `WriteRef` from a state, enabling users to modify the state after attaching it to a widget. (#601 @M-Adoo)
- **core**: Introduced the `DeclareInto` trait for any type that implements `DeclareFrom`. (#604 @M-Adoo)
- **macros**: Improved widget declaration to allow specifying widget types via path. (#606 @M-Adoo)
  ```rust
    // Previously, a widget type could only be specified using an identifier, requiring prior import of `Row`.
    use ribir::prelude::*;
    fn_widget! {
      @Row {
        ...
      }
    }

    // Now, the widget type can be specified using a path, removing the need for a prior import.
    fn_widget! {
      @ribir::prelude::Row {
        ...
      }
    }
   ```

### Changed

- **core**: Render widgets no longer need to implement the `Query` trait. Data can only be queried if it's a state or wrapped with `Queryable`. (#601 @M-Adoo)

### BREAKING

- **core**: Removed the infrequently used `StateFrom` trait, as there's a more efficient alternative. (#604 @M-Adoo)


## [0.4.0-alpha.1](https://github.com/RibirX/Ribir/compare/ribir-v0.3.0-beta.2...ribir-v0.4.0-alpha.1) - 2024-06-12

### Changed

- **core**: Removed the unused stamp checker for the split state. (#599 @M-Adoo)

## [0.3.0-beta.2](https://github.com/RibirX/Ribir/compare/ribir-v0.3.0-alpha.5...ribir-v0.3.0-beta.2) - 2024-06-05

We're thrilled to announce that Ribir now supports the Web platform\! 🎉🎉🎉

Experience the power of compiling Rust code to wasm and rendering it with WebGPU or WebGL.

Check out our Wordle game demo, now running smoothly in your browser\!

[![Wordle Game](./static/wordle-wasm.png)](https://ribir.org/wordle_game/)

### Features

- **ribir**: support stable Rust 1.77.0 (\#552 @M-Adoo)
- **macros**: Added a `include_crate_svg!` macro to include the svg relative to current crate. (\#552, @M-Adoo)
- **ribir**: Added a `nightly` feature to enable functionalities that require nightly Rust. (\#552, @M-Adoo)
  - The `include_crates_svg!` macro can operate without the `nightly` feature.
  - The `include_svg!` macro requires the `nightly` feature to be enabled.

- **ribir**: Introduced `AppRunGuard` to allow app and window configuration prior to app startup. (\#565, @M-Adoo)
  Previously, to configure the app and window before startup, `App::run` couldn't be used:
  
  ``` rust
  unsafe {
    AppCtx::set_app_theme(material::purple::light());
  }
  
  App::new_window(root, None).set_title("Counter");
  App::exec();
  ```
  
  Now, with AppRunGuard, you can use `App::run` and chain the configuration methods:
  
  ``` rust
  App::run(root)
    .with_app_theme(material::purple::light())
    .with_title("Counter");
  ```

- **core**: The split functions in `StateReader::map_reader`, `StateWriter::map_writer`, and `StateWriter::split_writer` no longer need to return a reference. (\#568 @M-Adoo)
- **core**: Introduced `StateWatcher` for watching state modifies, which was previously the responsibility of `StateReader`. This results in a cleaner and more compact `StateReader` implementation. (\#556, @M-Adoo)
- **gpu**: Introduced `GPUBackendImpl::max_textures_per_draw` to set a limit on textures per draw phase (\#562 @M-Adoo)
- **gpu**: Updated the `wgpu` implementation of the GPU backend to support WebGL. (\#578, @M-Adoo)
- **ci**: add wasm test (\#583 @wjian23)
- **ci**: wasm server watch file change (\#586 @wjian23)
- **painter**: Introduced support for `Resource<Path>` for drawing. This indicates that the `Path` may be shared with others, allowing the backend to cache it. (\#589 @M-Adoo)
- **painter**: Introduced support for bundled commands, enabling the backend to process these commands as a single entity and cache the resulting output. (\#589 @M-Adoo)

### Fixed

- **examples**: fix crash issue in Storybook (\#559 @M-Adoo)

- **ribir**: Resolved the issue causing a black screen on the first frame. (\#566, @M-Adoo)

- **gpu**: Retrieve the texture limit size from the GPU instead of using a hardcoded value. (\#578, @M-Adoo)
- **ribir**: fixed the crash issue when the shell window is zero-sized at startup. (\#582, @M-Adoo)

### Changed

- **core**: Enhanced panic location tracking during widget build (\#559 @M-Adoo)

- **core**: rename builtin field of delay\_drop\_until to keep\_alive (\#561 @wjian23)

- **macros**: polish the compile error message of invalid filed in `@$var {}` (\#556 @M-Adoo)
- **gpu**: Removed dependency on the texture array feature of wgpu. (\#562, @M-Adoo)
- **algo**: removed `Resource` and rename `ShareResource` to `Resource`. (\#564, @M-Adoo)
- **dev-helper**: Support specific the comparison of image tests. (\#573 @M-Adoo)
- **dev-helper**: If test images differ, both actual and difference images are saved with the expected image. (\#573 @M-Adoo)
- **painter**: Removed the AntiAliasing feature from the `painter` package, This responsibility now lies with the painter backend. (\#584 @M-Adoo)
- **gpu**: The GPU backend no longer relies on MSAA, which is dependent on the graphics API. Instead, it uses the alpha atlas to provide a solution similar to SSAA.(\#584, @M-Adoo)
- **example**: run example in web wasm (\#571 @wjian23)
- **gpu**: The GPU backend now only caches the path command if it is a `Resource`. This change reduces GPU memory usage and accelerates cache detection. (\#589 @M-Adoo)
- **text**: Implemented caching of the glyph path as a `Resource` to improve performance. (\#589 @M-Adoo)

### Documented

- **core**: Explained when to use `unsubscribe` with `watch!`. (\#556, @M-Adoo)

### Breaking

- **ribir**: compile wasm (\#543 @wjian23)

- **ribir**: Updated `App::new_window` to accept `WindowAttributes` instead of size as the second parameter. (\#565, \#566, @M-Adoo)
- **ribir**: The window creation APIs have been updated to use asynchronous methods, improving compatibility with browsers. (\#565, @M-Adoo)

- **macros**: removed `map_writer!` and `split_writer!` macros. (\#568, @M-Adoo)
- **ribir**: `StateWriter::map_writer` and `StateWriter::split_writer` now only require a writer split function, enhancing both reader and writer split operations. (\#568, @M-Adoo)
- **core**: The `StateReader` no longer supports watching its modifications. Use the `StateWatcher` trait instead for this functionality. (\#556 @M-Adoo)
- **painter**: Changes to `BackendPainter` APIs. This only affects you if you've implemented a custom painter. (\#562 @M-Adoo)

## [0.2.0](https://github.com/RibirX/Ribir/compare/ribir-v0.1.0...ribir-v0.2.0) - 2024-05-29

### Documented

- fix broken links and format the example code (\#526 @M-Adoo)
- **ribir**: We no longer auto-generate the built-in list document, as `FatObj` lists all. Its API documentation is sufficient. (\#540 @M-Adoo)
- **ribir**: Added guide "Using Ribir without 'DSL'" (\#545 @M-Adoo)
- **ribir**: Added a roadmap. (\#550, @M-Adoo)

### Breaking

While these are public APIs, they are typically not required for direct use in user code.

- **core**: removed `Stateful::on_state_drop` and `Stateful::unsubscribe_on_drop` (\#539 @M-Adoo)
- **core**: removed `AppCtx::add_trigger_task` and `AppCtx::trigger_task` (\#539 @M-Adoo)
- **core**: removed `FatObj::unzip` and `FatObj::from_host` (\#535 @M-Adoo)
- **core**: removed `BuiltinObj`. (\#535 @M-Adoo)
- **core**: `FatObj::new(host: T, builtin: BuiltinObj)` -\> `FatObj::new(host: T)`
- **core**: rename `DeclareBuilder` to `ObjDeclarer` (\#547 @M-Adoo)
- **core**: rename `DeclareBuilder::build_declare` to `ObjDeclarer::finish` (\#547 @M-Adoo)
- **core**: rename `Declare::declare_builder` to `Declare::declarer` (\#547 @M-Adoo)
- **core**: Renamed the `widget_build` method to `build` for brevity, given its frequent usage. (\#549 @M-Adoo)

### Features

- Support the overlay (@wjian23).
  
  This enhancement simplifies the creation of overlay widgets. It streamlines the addition of any widget to an overlay and offers a more user-friendly API for overlay management

- **macros**: Generates documentation for the builder methods of members in `#[derive(Declare)]`, thus improving IDE support.(\#538 @M-Adoo)

- **core**: All built-in widget abilities are now exported on `FatObj`. (\#535 @M-Adoo)
  You can directly use `FatObj` to configure built-in widget abilities such as `on_click`, `on_key_down`, etc.
  
  ``` rust
  let _ = FatObj::new(Void)
    .margin(EdgeInsets::all(1.0))
    .on_click(|_, _| { println!("click"); });
  ```

- **macros**: `#[derive(Decalre)]` now generates a `FatObj<State<T>>` instead of `State<T>`, and supports initialization of all built-in widgets on its ObjBuilder. (\#535 @M-Adoo)
  All pipes used to initialize the field will be unsubscribed when the FatObj is disposed.
  
  ``` rust
  let row = Row::builder()
    .margin(...)
    .on_click(...)
    .finish(ctx);
  ```

- **macros**: Introduced `simple_declare` macro for types that don't use `Pipe` for initialization. (\#535 @M-Adoo)

### Changed

- **core**: StateReader now automatically unsubscribes when no writer is present (\#532 @wjian23)
- **core**: Consolidated all listener and `FocusNode` into a `MixBuiltin` widget (\#534 @M-Adoo)
  - The `MixBuiltin` widget reduces memory usage and allows users to utilize all `on_xxx` event handlers, not only during the build declaration but also after the widget has been built.
- **core**: removed `MixBuiltinDeclarer`, which is no longer needed. (\#538 @M-Adoo)
- **macros**: removed crate `ribir_builtin` that is no longer needed. (\#535 @M-Adoo)

## [0.1.0](https://github.com/RibirX/Ribir/releases/tag/ribir-v0.1.0) - 2024-03-26

🎉🎉🎉 The first version of Ribir.

![background](./static/hero-banner.png)

The goal of this version of Ribir is to finish the core framework and answer our questions about the feasibility of the design.

We use it to build examples and build some apps for our daily work. And we are satisfied with the experience of using it.

We are very happy to share it with you. We hope you can try it out and give us feedback. But we don't recommend you to use it in production environments yet.

### Features

- **core**: control the process of the entire view: compose, build, update, layout and render.
- **declarative language**: not a new language, but a set of Rust macros that easily interact with Rust.
- **widgets compose system**: has four kinds of widgets to support you can implement your own widget in different ways:
  - function widget and `Compose`, from other widgets composition.
  - `Render`, implement your layout and paint anything you want.
  - `ComposeChild`, control the compose logic between parent and child widgets and specify the template of child widgets.
- **non-intrusive state**: convert your data to a listenable state, and update the view according to the change of the state.
- **layout system**: learning and inspired by [Flutter](https://flutter.dev/) Sublinear layout, but not the same.
- **event system**: a composition event system, that supports event bubbling and capture. Allow to compose with any widget, and exists only if you use it.
- **theme System**: support full and inherit/partial theme, so you can use it to override or dynamically switch the theme of the subtree. Include palette, icons, animate transitions, the decoration widget of the widget, etc. In a very rough state and the API will be redesigned soon.
- **animations**: based on state but no side effect animation, it's almost stable in concept, but not many predefined animations yet.
- **painter**： convert the view to the 2D path.
- **GPU render**: GPU backend for the **painter**, do path tessellation, so that easy to render the triangles in any GPU render engine. A `wgpu` implementation is provided as the default GPU render engine. Tessellation base on [lyon](https://github.com/nical/lyon).
- **text**: support basic text typography and IME input, in a usable but rough stage.
- **widgets**: the widgets library provides 20+ basic widgets, but all are in a rough stage, and the API is not stable yet.
- **examples**: counter, storybook, messages, todos, wordle\_game, etc.

### Documented

- **introduction**: add `introduction.md` to introduce Ribir and why choose it.
- **get started**: add the `get_started` series of tutorials to help users get started with Ribir.

<!-- next-url -->
[@Unreleased]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.9...HEAD
[0.4.0-alpha.9]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.8...ribir-v0.4.0-alpha.9
[0.4.0-alpha.8]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.7...ribir-v0.4.0-alpha.8
[0.4.0-alpha.7]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.6...ribir-v0.4.0-alpha.7
[0.4.0-alpha.6]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.5...ribir-v0.4.0-alpha.6
[0.4.0-alpha.5]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.4...ribir-v0.4.0-alpha.5
[0.4.0-alpha.4]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.3...ribir-v0.4.0-alpha.4
[0.4.0-alpha.3]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.2...ribir-v0.4.0-alpha.3
[0.4.0-alpha.2]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.1...ribir-v0.4.0-alpha.2
