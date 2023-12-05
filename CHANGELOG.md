# Changelog

All notable changes to this project will be documented in this file.

Please keep one empty line before and after all headers. (This is required for `git` to produce a conflict when a release is made while a PR is open and the PR's changelog entry would go into the wrong section).

There are 5 types of changes:

- `Features` for new features.
- `Changed` for changes in existing functionality.
- `Fixed` for any bug fixes.
- `Documented` for any changes to the documentation.
- `Breaking` for the detail of any backwards incompatible changes.

And please only add new entries below the [Unreleased](#unreleased---releasedate) header with the following format:

```md
**crate or effect scope**: description of change ([#PR])
```

<!-- next-header -->

## @Unreleased - @ReleaseDate

🎉🎉🎉 The first version of Ribir. 

![background](https://not.ready/demos.png)

As the first version, its main content is to verify and stabilize the basic concepts, determine the overall framework process, and make preliminary attempts to verify all core modules. 

We use it in our own projects and have a good experience, and we hope you can also try it out and give us feedback. But it is still in a very rough stage, and be careful to use it in production.

### Features

- **core**: control the process of the entire view: compose, build, update, layout and render.

- **declarative language**: not a new language, but a set of Rust macros that easily interact with Rust.

- **widgets compose system**: has four kinds of widgets to support you can implement your own widget in different ways:
: function widget, `Compose`, `Render` and `ComposeChild`. So that
  - function widget and `Compose`, from other widgets composition.
  - `Render`, implement your own layout or rendering logic
  - `ComposeChild`, control the compose logic between parent and child widgets, and specify the template of child widgets.  

- **non-intrusive state**: convert your data to a listenable state, and update the view according to the change of the state.

- **layout system**: learning and inspired by [Flutter] Sublinear layout, but not exactly the same.

- **event system**: a compose event system, support event bubbling and capture. Support compose to any widget, and exist only if you use it.

- **theme System**: support full and inherit/partial theme, so you can use it to override or dynamically switch the theme of the subtree. Include: palette, icons, animate transitions, the decoration widget of the widget, etc. In a very rough state and the API will be redesigned soon.

- **animations**: base on state but no side effect animation, it's almost stable in concept, but not many predefined animations yet.

- **painter**： convert view to 2d path.

- **gpu render**: gpu backend for **painter**, do path tessellation, so that easy to render the triangles in any gpu render engine. A `wgpu` implementation is provided as the default gpu render engine. Tessellation base on [lyon].

- **Text**: support basic text typography and ime input, in a usable but rough stage.

- **widgets**: widgets library provide 20+ basic widgets, but all in a rough stage, and the API not stable yet.

- **examples**: counter, storybook, messages, todos, wordle_game, etc.

### Documented

- **introduction**: add `introduction.md` to introduce Ribir and why choose it.
- **get started**: add the `get_started` series of tutorials to help user get started with Ribir.


[Flutter]: https://flutter.dev/
[lyon]: https://github.com/nical/lyon

<!-- next-url -->


