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

- **core**: Introduced `IntoWidget` and `IntoChild`. (@M-Adoo #pr)

  The `IntoWidget` trait allows for the conversion of any widget to the type `Widget`.
  The `IntoChild` trait provides a way to convert a more general widget into a child of `ComposeChild`.

### Changed

- **core**: Simplify the implementation of parent composition with child widgets. (#pr, @M-Adoo)
  
  Merge `SingleWithChild`, `MultiWithChild`, and `ComposeWithChild` into a single trait called WithChild.

### Breaking

- Remove SingleParent and MultiParent traits. (#pr, @M-Adoo)
- Allow only the child to be converted to a widget or a type that implements the Into trait. (#pr, @M-Adoo)


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
[@Unreleased]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.3...HEAD
[0.4.0-alpha.3]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.2...ribir-v0.4.0-alpha.3
[0.4.0-alpha.2]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.1...ribir-v0.4.0-alpha.2
