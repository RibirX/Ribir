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
**crate or effect scope**: description of change (#PR @contributor)
```

<!-- next-header -->

## [@Unreleased] - @ReleaseDate

### Documented

- **Ribir**: Added a roadmap. (#550, @M-Adoo)


## [0.2.0-alpha.7] - 2024-03-19

### Breaking

_ **core**: rename `Declare::declare_builder` to `Declare::declarer` (#545 @M-Adoo)
- **core**: rename `DeclareBuilder` to `ObjDeclarer` (#545 @M-Adoo)
- **core**: rename `DeclareBuilder::build_declare` to `ObjDeclarer::finish` (#545 @M-Adoo)

## [0.2.0-alpha.6] - 2024-03-12

### Features

- **macros**: Generates documentation for the builder methods of members in `#[derive(Declare)]`, thus improving IDE support.(#538 @M-Adoo)
- **core**: All built-in widget abilities are now exported on `FatObj`. (#535 @M-Adoo)
  You can directly use `FatObj` to configure built-in widget abilities such as `on_click`, `on_key_down`, etc.
  ```rust
  let _ = FatObj::new(Void)
    .margin(EdgeInsets::all(1.0))
    .on_click(|_, _| { println!("click"); });
  ```
- **macros**: `#[derive(Decalre)]` now generates a `FatObj<State<T>>` instead of `State<T>`, and supports initialization of all built-in widgets on its ObjBuilder. (#535 @M-Adoo) 
  All pipes used to initialize the field will be unsubscribed when the FatObj is disposed.
  ```rust
  let row = Row::builder()
    .margin(...)
    .on_click(...)
    .finish(ctx);
  ```
- **macros**: Introduced `simple_declare` macro for types that don't use `Pipe` for initialization. (#535 @M-Adoo)

### Changed

- **core**: removed `MixBuiltinDeclarer`, which is no longer needed. (#538 @M-Adoo)
- **macros**: removed crate `ribir_builtin` that is no longer needed. (#535 @M-Adoo)

### Documented

- **ribir**: We no longer auto-generate the built-in list document, as `FatObj` lists all. Its API documentation is sufficient. (#540 @M-Adoo)


### Breaking

- **core**: removed `Stateful::on_state_drop` and `Stateful::unsubscribe_on_drop` (#539 @M-Adoo)
- **core**: removed `AppCtx::add_trigger_task` and `AppCtx::trigger_task` (#539 @M-Adoo)
- **core**: removed `FatObj::unzip` and `FatObj::from_host` (#535 @M-Adoo)
- **core**: removed `BuiltinObj`. (#535 @M-Adoo)
- **core**: `FatObj::new(host: T, builtin: BuiltinObj)` -> `FatObj::new(host: T)`

While these are public APIs, they are typically not required for direct use in user code.


## [0.2.0-alpha.5] - 2024-03-05

### Features

- Support the overlay (@wjian23).

   This enhancement simplifies the creation of overlay widgets. It streamlines the addition of any widget to an overlay and offers a more user-friendly API for overlay management

## [0.2.0-alpha.4] - 2024-02-27

### Changed

- **core**: StateReader now automatically unsubscribes when no writer is present (#532 @wjian23)
- **core**: Consolidated all listener and `FocusNode` into a `MixBuiltin` widget (#534 @M-Adoo)
  - The `MixBuiltin` widget reduces memory usage and allows users to utilize all `on_xxx` event handlers, not only during the build declaration but also after the widget has been built.

## [0.2.0-alpha.3] - 2024-02-20

## [0.2.0-alpha.2] - 2024-02-13

### Documented

- fix broken links and format the example code (#526 @M-Adoo)

## [0.1.0-beta.7](https://github.com/RibirX/Ribir/compare/ribir-v0.1.0-alpha.0...ribir-v0.1.0-beta.7) - 2024-02-02

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

<!-- next-url -->
[@Unreleased]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.7...HEAD
[0.2.0-alpha.7]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.6...ribir-v0.2.0-alpha.7
[0.2.0-alpha.6]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.5...ribir-v0.2.0-alpha.6
[0.2.0-alpha.5]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.4...ribir-v0.2.0-alpha.5
[0.2.0-alpha.4]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.3...ribir-v0.2.0-alpha.4
[0.2.0-alpha.3]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.2...ribir-v0.2.0-alpha.3
[0.2.0-alpha.2]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.1...ribir-v0.2.0-alpha.2

- **introduction**: add `introduction.md` to introduce Ribir and why choose it.
- **get started**: add the `get_started` series of tutorials to help users get started with Ribir.
