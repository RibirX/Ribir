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

### Fixed

- Add Api to check StateWriter valid (#521 @wjian23)


## [0.2.0-alpha.4] - 2024-02-27

### Fixed

- Optimization, StateReader auto unsubscribe if not writer(#532 @wjian23)

### Changed

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
[@Unreleased]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.4...HEAD
[0.2.0-alpha.4]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.3...ribir-v0.2.0-alpha.4
[0.2.0-alpha.3]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.2...ribir-v0.2.0-alpha.3
[0.2.0-alpha.2]: https://github.com/RibirX/Ribir/compare/ribir-v0.2.0-alpha.1...ribir-v0.2.0-alpha.2

- **introduction**: add `introduction.md` to introduce Ribir and why choose it.
- **get started**: add the `get_started` series of tutorials to help users get started with Ribir.
