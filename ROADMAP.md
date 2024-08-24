# Ribir Roadmap

Check out the [Milestones](https://github.com/RibirX/Ribir/milestones) to stay updated with our current projects and upcoming releases.

Refer to the [Releases](./RELEASE.md) for information on our release schedule and our process for publishing new versions.


## Prototyping (v0.1 done, 2024 Feb)

This milestone aimed to build the core framework and test if the design works. We tried to finish most things, but they didn't have to be perfect. 

After this milestone, we should be confident that we can create a high-performance, user-friendly native GUI framework. The core ideas should be stable.

- [x] Macros for more readable view code
- [x] Simple state system for efficient and intuitive view updates
- [x] Basic widget support, including composition and rendering
- [x] Layout system
- [x] Event listener attachment
- [x] Theme support, including partial themes in subtrees
- [x] GPU rendering and interchangeable painters
- [x] Animation support
- [x] Basic widget library for examples
- [x] Basic text and IME input support
- [x] Prepare basic guides and examples
- [x] Support for desktop platforms (Linux, Windows, macOS)

## Improve Core API (v0.2 done, March 2024)

This milestone aimed to improve the core API, making it easier for users to create their own widgets and applications, either directly or using macros.

- [x] Add simple API for overlay support
- [x] Make APIs for extending built-in widgets more user-friendly
- [x] Improve widget creation API names
- [x] Fix circular reference issue when using `pipe` to set widget properties
- [x] Provide guide on using Ribir without macros

## Web Compatibility and widget system APIs Stabilization (v0.3, April-May 2024)

This milestone aims to prepare Ribir for the web and stabilize the widget system APIs.

- [x] Switch to stable Rust
- [x] Add support for browsers (WASM + WebGPU/WebGL)
- [x] Make state splitter support any type with a lifetime. Ribir will implement `&T`, `Option<&T>`, and `Result<&T, E>` by default.
- [x] Ensure only `watch!` need to be manually unsubscribed and provide a guide on how to do so

## Theme API Stabilization and State Provider (v0.4, June 2024)

This milestone aims to stabilize the theme API, simplify type conversion, and facilitate the development of widgets with dynamic themes.

- [x] Simplify type conversion.
  We've over-engineered some aspects of type conversion, which has actually increased the learning curve for users and reduced error readability. For instance, the conversion of `DeclareInit` and the nested conversion of `Template`. The downside is that more explicit conversions will be required when using them.
- [x] Implement a provider widget that can exports a state to its subtree, allowing widgets in its subtree to query the state using context.
- [x] Simplify the theme system API to enhance user-friendliness.
- [ ] Include additional built-in paint style widgets that will be inherited by descendants, such as `TextStyle` and `Foreground`.
- [ ] Implement a mechanism to enable sharing styles between widgets, akin to the `class` attribute in HTML.

## Widgets Library And Storybook (v0.5)

- [ ] In-depth widget guide, explaining how widgets work and how to create custom widgets.
- [ ] Production level widgets library with the basic widgets
- [ ] Complete the basic and material themes
- [ ] storybook to display all widgets, allowing user interaction


## Backlog

The following tasks are in no particular order. When we start the next milestone, we will decide its content based on current needs and progress.

- [ ] Mobile platform support (iOS, Android)
  - Compiling should be straightforward, but usability will depend on multi-touch and gesture support.
- [ ] Multi-touch and gesture support.
- [ ] Tools
  - [ ] Development tools
  - [ ] Bundle tool
- [ ] Performance metrics and optimization
- [ ] Update rxRust to 1.0
- [ ] Try-Ribir App, a collection of all examples, guides, and widgets to help users learn Ribir.
- [ ] Text testing - bidi, rtl, vertical text, etc.
- [ ] Drag and drop support
- [ ] Provide more animations and attractive demos to showcase them