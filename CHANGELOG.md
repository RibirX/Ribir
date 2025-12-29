# Changelog

All notable changes to this project will be documented in this file.

This changelog is automatically generated from Pull Request descriptions.
See [CONTRIBUTING.md](CONTRIBUTING.md) for how to write changelog entries in your PR.

<!-- next-header -->

## [@Unreleased] - @ReleaseDate

### Features

- **core**: Add Image widget with lazy WebP decoding, caching, and animation support. (#823 @M-Adoo)
- **core**: Add a tool to run CI tests locally via `cargo +nightly ci`. (#822 @M-Adoo)

### Changed

- **core**: Refactor the scheduler to improve async ecosystem compatibility. (#815 @M-Adoo)
- **gpu**: Upgrade wgpu to 0.28.0 and adapt to API changes. (#820 @M-Adoo)
- **gpu**: Limit max filter kernel size to avoid macOS Metal hangs. (#820 @M-Adoo)

### Fixed

- **core**: Fix abnormal CPU usage on macOS caused by the side effects of cloning EventLoopProxy (#816 @wjian23).

### Documented

- **doc**: add guide for ribir. (#823 @wjian23)

## [0.4.0-alpha.53] - 2025-12-17

- **cli**: support both [bundle] and [package.metadata.bundle] config formats. (#814 @M-Adoo)

### Features

- **core**: add builtin `box_shadow` property. (#811 @wjian23)
- **core**: add builtin `filter` property. (#811 @wjian23)

## [0.4.0-alpha.52] - 2025-12-03

### Features

- **themes**: Material theme now registers icons to `svg_registry`. (#806 @M-Adoo)

- **widgets**: add Badge widget for showing notifications, counts, or status information on top of another widget.(#805 @wjian23)

### Breaking

- **themes**: Removed `IconTheme`, `fill_svgs!`, `svgs` and `material_svgs`. Use `svg_registry` to manage and access icons instead. (#806 @M-Adoo)

## [0.4.0-alpha.51] - 2025-11-26

### Features

- **macros**: add `asset!` macros for asset management.(#798 @M-Adoo)
- **macros**: add `include_asset!` macro for compile-time asset embedding. (#799 @M-Adoo)

- **widgets**: add Switch widget for toggling boolean states with Material Design styling and animations.(#804 @wjian23)

### Breaking

- **macros**: replace `include_crate_svg!` with new `asset!` macro for general asset management.(#798 @M-Adoo)


## [0.4.0-alpha.50] - 2025-11-19

### Fixed

- **core**: reduce the memory usage. (#797 @wjian23)
  - update wgpu from v24 to v27, create gpu backend with memory_hints of MemoryHints::MemoryUsage

- **core**: fix overlay close panic when window been closed. (#796 @wjian23)

### Features

- **widgets**: add changed event to Slider and Checkbox.(#796 @wjian23)
- **core**: add window positioning and level control APIs.(#796 @wjian23)
- **example**: add example of a Pomodoro timer app.(#796 @wjian23)

## [0.4.0-alpha.49] - 2025-09-03

## [0.4.0-alpha.48] - 2025-08-27

## [0.4.0-alpha.47] - 2025-08-20

### Features

- **cli**: add support for packaging Ribir projects, see README.md in cli for details. (#777 @wjian23)

### Fixed

- **gpu**: Fixed the missed submission of GPU commands for drawing mask triangles, which led to abnormal color filling. (#781 @wjian23)

### BREAKING

- **core**: Remove `State`; use `Stateful` instead. (#782 by @M-Adoo)
- **core**: Remove `TransitionTheme`. (#783 by @M-Adoo)

## [0.4.0-alpha.46] - 2025-08-13

### Fixed

- **gpu**: Fixed filter panic in some platform which Surface can't with TEXTURE_BINDING usage. (#780 @wjian23)

## [0.4.0-alpha.45] - 2025-08-06

### Features

- **core**: Added built-in property `backdrop_filter`. (#778 @wjian23)

### Fixed

- **gpu**: Fixed the drawn colors are biased. (#778 @wjian23)


## [0.4.0-alpha.44] - 2025-07-10

### Features

- **macros**: Supported parent expression syntax using `$(...) { }`. (#773 @M-Adoo)
- **macros**: builtin field names are not reversed. (#773 @M-Adoo)

### Changed

- **core**: Separate UI Rendering and Application Logic into Independent Threads. (@wjian23).
- **core**: remove CustomStyles and TextField Widget.(@wjian23)

### BREAKING

- **macros**: supported explicit '$' syntax for variable capture and state modifies (#773 @M-Adoo)
  - Requires using `$read()`, `$write()`, `$reader()`, `$writer()`, `$watcher()`, and `$clone()` for state operations
- **core**: Removed all `get_xxx_widget` methods from `FatObj`, built-in state now directly accessible via `FatObj` exports (#773 @M-Adoo)


## [0.4.0-alpha.40] - 2025-06-04

### Changed

- **core**: Refactored `Pipe` trait into a `Pipe` struct, simplifying pipe-type management. (#765 @M-Adoo)
- **core**: Added built-in `TextAlign` provider for text alignment via `text_align` property. (#764 @M-Adoo)

### BREAKING

- **core**: Refactor partial writer to use `PartialId` (#762 by @wjian23)  
  Changes include:
  - Renamed `map_reader` → `part_reader`
  - Renamed `map_watcher` → `part_watcher`
  - Merged `map_writer` and `split_writer` into a single `part_writer` method:
    * Now accepts `id: PartialId` parameter
    * Creates isolated child writers for specific data segments
    * Child writers ignore parent modifications
    * Parents control child modification propagation
- **core**: Field declaration methods must now start with `with_` prefix (#767 by @M-Adoo)
  - This change does not break the declaration syntax but introduces a breaking change for declarer APIs.
- **macros**: Replaced `@ $var { ... }` syntax with `@(expr) { ... }` to support expression parent and provide more uniform syntax. (#768 @M-Adoo)


## [0.4.0-alpha.39] - 2025-05-28


## [0.4.0-alpha.38] - 2025-05-21

### BREAKING

- **Widgets**: `Row` and `Column` now provide basic linear layouts, arranging children in a straight line without `Flex`'s advanced capabilities. (#759 @M-Adoo)


## [0.4.0-alpha.37] - 2025-05-14

### Features

- **painter**: Support jpeg image. (#753 @wjian23)

### Changed

- **core**: Simplify type conversion system by unifying implementations under `RFrom` and `RInto` traits (by @M-Adoo)


## [0.4.0-alpha.36] - 2025-05-07

### Features

- **core**: Added common fallback system fonts list function `fallback_font_families`, so that themes can use. (#748 @M-Adoo)

### BREAKING

- **core**: The `ComposeDecorator` trait has been removed. (#754 @M-Adoo)

## [0.4.0-alpha.35] - 2025-04-30

### Fixed
- **core**: Fixed the priority of the pipe does not depend on its position (the previous position was not accurately tracked). (#742 @wjian23)
- **core**: Fixed incorrect child positioning during `box_fit` scaling (#743 by @M-Adoo)
- **widgets**: Fixed infinite layout loop caused by `Scrollbar` during window resizing (#743  @M-Adoo)
- **widgets**: Prevent stack re-layout when subtree of InParentLayout changed.(#745 @wjian23)
- **widgets**: Constrain the size of Expanded when the remaining space for Expanded in Flex is zero(#747 @wjian23)

### BREAKING CHANGES
- **core**: method of Render `only_sized_by_parent` rename to `size_affected_by_child`, Widget `OnlySizedByParent` rename to `NoAffectedParentSize`.(#745 @wjian23)

## [0.4.0-alpha.34] - 2025-04-23

### Features

- **core**: Added `Location` provider to manage and track user navigation positions within application windows. (#740 @M-Adoo)
- **widgets**: Added `Router` widget to handle navigation within window. (#746 @M-Adoo)

### Fixed
- **core**: Fixed `Reuseable` panic when when holding the same Reuseable while pipe changes continuously.(#741 @wjian23)

### BREAKING CHANGES
- **core**: Added built in filed `reuse_id`, `GlobalWidgets` and `LocalWidgets`; removed `KeyWidget`. Use `reuse_id` as replacements for similar scenarios.(#741 @wjian23)

## [0.4.0-alpha.33] - 2025-04-16

### Features

- **core**: Added `Reusable` helper to enable widget recycling and reuse. (#737 @M-Adoo)


## [0.4.0-alpha.32] - 2025-04-09

### Features

- **core**: Added built-in method `focus_change_reason` to retrieve last focus/blur event cause. (#734 by @M-Adoo)
- **core**: Added reason in `FocusEvent` to indicate the cause of focus change. (#734 @M-Adoo)
- **painter**: Added `fork()`/`merge()` methods for `Painter` for low-cost layer composition (#736 @M-Adoo)
- **material**: Added `InteractiveLayers` to provide material design interactive layers for its child. (#736 @M-Adoo)



### Fixed

- **core**: Fix `FittedBox` to always center its child (#727 by @M-Adoo)
- **core**: Fix incorrect drop pointer press or release event when multiple devices are used simultaneously, ensuring that only the tap event is dropped while other events are fired correctly. (#730 @M-Adoo)
- **core**: Fix the bug where the outer data of the first child is incorrectly wrapped around each child when the pipe dynamically generates multiple children. (#735 @wjian23)


### Changed

- **widgets**: The `List` widgets has been redesigned with class-based styling and cleaner syntax. (#727 @M-Adoo)
- **ribir**: Updated winit dependency to v0.30.* (#728 @M-Adoo)
- **ribir**: Updated wgpu dependency to v0.24.* (#728 @M-Adoo)


### BREAKING CHANGES

- **core**: Standardize built-in method naming conventions: (#734 @M-Adoo)
  - Boolean state checks (past participle):
    - `has_focus` → `is_focused`
    - `is_hover` → `is_hovered`
  - Property accessors:
    - `is_auto_focus` → `auto_focus` (getter)
- **ribir/web**: Changed canvas management strategy: (#728 @M-Adoo)
  - Now searches for `ribir_container` element to append new canvas
  - No longer reuses existing canvas elements


## [0.4.0-alpha.31] - 2025-04-02

### Features

- **core**: Introduced the `PairOf` utility to preserve parent and child type information of `ComposeChild`. (#724 @M-Adoo)
- **core**: Added `class_array!` macro to apply multiple classes at once. (#724 by @M-Adoo)
- **core**: Added non-child field support to `Template`. (#725 @M-Adoo)
  - Support default values for non-child fields via `#[template(field)]` attribute.
  - Maintain backward compatibility with existing child-focused patterns


### Breaking

- **core**: PipeWidget will be lazy created by pipe value of FnWidget.(#723 @wjian23)
- **core**: Changed `Declare` to initialize fields in-place rather than returning a new object (#724 by @M-Adoo)
- **core**: The fn_widget! macro preserves the type information of the returned widget. (#726 @wjian23)
- **core**: Refactor KeyWidget, reuse the instance with same key when Pipe regenerate. (#726 @wjian23)


## [0.4.0-alpha.30] - 2025-03-26

### Features

- **theme**: Added support for the Material Theme using the `DisabledRipple` provider to disable the ripple effect. (#722 by @M-Adoo)
- **widgets**: Added `defer_alloc` to `Expanded` widget, allowing space allocation to be deferred until after other widgets are allocated. (#722 @M-Adoo)

### Fixed

- **macros**: Fixed `part_xxx!` macro handling of built-in widget state when used as a top-level macro. (#722 @M-Adoo)

## [0.4.0-alpha.29] - 2025-03-19


### Features

- **core**: record the visual rect after layout.(#698 @wjian23)
- **widget**: Added the Widget of Menu.(#702 @wjian23)
- **widgets**: Allow children of the `Stack` to adjust their size based on the `Stack`'s size. (#706 @M-Adoo)
- **core**: Added support for ColorFilter.(#709 @wjian23)
- **core**: Added builtin field `disable`.(#712 @wjian23)
- **core**: Added ColorFilter of `hue_rotate_filter` and `saturate_filter`(#712 @wjian23)

### Fixed

- **core**: Fixed the provider of the current widget was lost during event bubbling.(#702 @wjian23)
- **core**: Fixed the panic when overlay recreate provider_ctx during event callback.(#702 @wjian23)
- **core**: fix miss pop providers when call `push_providers_for` separately during layout.(#698 @wjian23)
- **core**: enables embedding declare template as child in widget with Vec of template(#704 @wjian23)
- **core**: Ensure layout event is emitted once per layout phase (not per frame) (#708 by @M-Adoo)
- **core**: Use minimum constraint size for viewport in unbounded layouts (#708 by @M-Adoo)
- **painter**: Properly discard render operations when clipping to zero-sized rectangles (#708 @M-Adoo)
- **macro**: Fixed issue where top-level `fn_widget!` macro did not capture a built-in widget. (#706 @M-Adoo)
- **core**: Fixed track_id in class node not update.(#712 @wjian23)
- **core**: Fixed FocusScope not work when host changed by class or pipe.(#712@wjian23)

### Changed

- **widgets**: Refactor `Divider` Widget. (#702 @wjian23)
- **widgets**: Refactor `Tabs` Widget. (#707 @M-Adoo)
- **widgets**: Refactor `Avatar` Widget. (#714 @M-Adoo)
- **ci**: update rust version of ci to 2025-03-06 (#702 @wjian23)

### Breaking

- **core**: Remove the `text_align` from the `Text` widget and replace it with the `TextAlign` provider instead. (#706 @M-Adoo)

## [0.4.0-alpha.27] - 2025-02-12

### Features

- **core**: Added the ability to force redraw a frame. (#697 @zihadmahiuddin)


### Fixed

- **core**: Fix window staying empty after switching workspace (e.g. in i3wm) by doing a force redraw. (#697 @zihadmahiuddin)

## [0.4.0-alpha.26] - 2025-02-05

### Fixed

- **widgets**: Ensure that the `Flex` expands items only after allocating space to all items, prioritizing the display of items in full initially. (#696 @M-Adoo)


## [0.4.0-alpha.25] - 2025-01-29

### Features

- **core**: Added builtin field `clip_boundary`. (#694 @wjian23)
- **core**: `IgnorePointer` now has the ability to only ignore events for the widget itself. (#695 @M-Adoo)
- **core**: Included `BoxPainter` to draw decorations starting from the widget box's origin while ignoring padding. (#695 @M-Adoo)

### Changed 

- **macros**: Generate cleaner code for #[derive(Declare)] when all fields are omitted. (#695 @M-Adoo)

### Fixed

- **core & widgets**: Layouts are not permitted to return an infinite size, so if a layout requires scaling or expanding the size infinitely, that action should be disregarded. (#695 @M-Adoo)
- **macros**: Embedding `fn_widget!` may lead to missed captured variables. (#695 @M-Adoo)
- **core**: The child should not be painted when visible is false. (#695 @M-Adoo)
- **core**: Ensure that the content widget size in the scrollable widget is not smaller than its viewport. (#695 @M-Adoo)
- **core**: The crash occurs when a parent widget with a class tries to convert the widget with more than one leaf. (#695 @M-Adoo)
- **core**: The padding only reduces the content size and does not affect the border and background size. (#695 @M-Adoo)

## [0.4.0-alpha.24] - 2025-01-22

### Features

- **core**: ensure object safety of `StateReader`， `StateWatcher` and `StateWriter` (#692 @M-Adoo)
- **core**: Support extend custom event. (#684 @wjian23)
- **core**: Added `part_watcher` to `StateWatcher` (#684 @wjian23)
- **core**: Added `visible_widget` and ScrollableProvider to ScrollableWidget, to support descendant to be showed.(#684 @wjian23)

### Changed

- **core**: Unified implementation of IntoWidget for impl StateWriter<V:Compose>. (#684 @wjian23)
- **widgets**: Refactor `Input` Widget. (#684 @wjian23)

### Breading

- **core**: Rename `can_focus` field of FocusScope to `skip_host`. (#684 @wjian23)

## [0.4.0-alpha.23] - 2025-01-15

### Features

- **core**: The `Render::dirty_phase` method has been added to allow widgets to mark only the paint phase as dirty when it is modified. (#689 @M-Adoo)
- **core**: Supports `Provider` to dirty the tree if it's a state writer. (#689 @M-Adoo)
- **core**: Added the built-in field `providers` to provide data to its descendants. (#690 @M-Adoo)
- **core**: Added `Variant` to support building widgets with variables across `Providers`. (#690 @M-Adoo)
- **macros**: Added the `part_reader!` macro to generate a partial reader from a reference of a reader. (#688 @M-Adoo)
- **macros**: The `simple_declare` now supports the `stateless` meta attribute, `#[simple_declare(stateless)]`. (#688 @M-Adoo)

### Changed

- **widgets**: Replace `BoxDecoration` with three separate widgets: `BorderWidget`, `RadiusWidget`, and `Background`. (#691 @M-Adoo)

### Fixed

- **Core**: `PartData` allows the use of a reference to create a write reference, which is unsafe. Introduce `PartRef` and `PartMut` to replace it. (#690 @M-Adoo)


### Breading

- **core**: Removed `PartData`. (#690 @M-Adoo)
- **core**: Removed `BoxDecoration`. (#691 @M-Adoo)

## [0.4.0-alpha.22] - 2025-01-08

### Fixed

- cargo: Fixed Documentation link (#686 @EpixMan)

## [0.4.0-alpha.21] - 2025-01-01

### Fixed

- **core**: The animation finish may miss drawing the last frame. (#682 @M-Adoo)


## [0.4.0-alpha.20] - 2024-12-25

### Features

- **core**: Added `Measure` to enable support for percentage values for position, and `Anchor` now supports percentage values. (#672 @M-Adoo)
- **core**: Added APIs `Window::once_next_frame`, `Window::once_frame_finished` and `Window::once_before_layout`. (#672 @M-Adoo)
- **painter**: Typography now supports baselines (middle and alphabetic). (#674 @M-Adoo)

### Changed

- **widgets**: Refactor the buttons use class to implement. (#675 @M-Adoo)

### Fixed

- **core**: Fix set opacity zero no work to it's children. (#671 @wjian23)
- **core**: Fix TextStyle cause providers mismatched (#671 @wjian23)
- **core**: Running an animation that is already in progress does not trigger a smooth transition. (#672 @M-Adoo)
- **core**: The framework incorrectly clamps the layout result of the render widget. (#672 @M-Adoo)
- **core**: Allow children to be hit outside their parent's boundaries for non-fixed-size containers. (#676 @M-Adoo)
- **painter**: Fixed text line height does not work correctly. (#674 @M-Adoo)
- **painter**: Fixed issue with text not being drawn at the middle baseline by default. (#674 @M-Adoo)

### Changed

- **core**: Optimize QueryId::is_same by not creating a String using format for every comparison (#678 @tashcan)

## [0.4.0-alpha.19] - 2024-12-18

### Features

- **core**: Added `grab_pointer` to grabs all the pointer input. (#669 @wjian23)
- **widgets**: Added the widget of Slider (#669 @wjian23)

### Fixed

- **core**: Fix mismatch of providers. (#669 @wjian23)
- **core**: Added DeclarerWithSubscription to let Widget `Expanded` accept pipe value. (#669 @wjian23)

## [0.4.0-alpha.18] - 2024-12-11

### Features

- **core**: Enhanced support for built-in fields such as `font_size`, `font_face`, `letter_spacing`, `text_line_height`, and `text_overflow` through `TextStyleWidget`. (#668 @M-Adoo)
- **widgets**: Icon size should be maintained even if its container is not sufficiently large. (#668 @M-Adoo)
- **core**: Added the builtin widget of tooltips (#664 @wjian23)

### Changed

- **core**: Refactor the builtin widget of global_anchor (#664 @wjian23)

## [0.4.0-alpha.17] - 2024-12-04

### Features

- **core**: Added the `named_svgs` module to enable sharing SVGs using string keys, replacing the need for `IconTheme`. (#658 @M-Adoo)
- **core**: The `keyframes!` macro has been introduced to manage the intermediate steps of animation states. (#653 @M-Adoo)
- **core**: Added `QueryId` as a replacement for `TypeId` to facilitate querying types by Provider across different binaries. (#656 @M-Adoo)
- **widgets**: Added `LinearProgress` and `SpinnerProgress` widgets along with their respective material themes. (#630 @wjian23 @M-Adoo)
- **painter**: SVG now supports switching the default color, allowing for icon color changes. (#661 @M-Adoo)

### Changed

- **widgets**: The `Checkbox` widget uses classes to style and simplify its label syntax. (#666 @M-Adoo)
- **widgets**: The `Icon` widget utilizes classes to configure its style, and it does not have a size property. (#660 @M-Adoo)
- **theme:** Refactor the `Ripple` and `StateLayer` of the material theme to enhance their visual effects. (#666 @M-Adoo)

### Fixed

- **core**: The size of the `Root` container is too small, which could lead to potential missed hits. (#654 @M-Adoo)
- **core**: The hit test for the `TransformWidget` is not applied at the correct position. (#654 @M-Adoo)
- **core**: Switching to a style class may result in missing widgets. (#655 @M-Adoo)
- **core**: The animation does not restore the state value correctly when multiple animations are applied to the same state. (#662 @M-Adoo)
- **macros**: the top-level `rdl!`, `pipe!` and `watch!` do not capture built-in widgets as intended. (#666 @M-Adoo)

- **core**: inner embed anchor not work (#665 @wjian23)
- **core**: fix query render object with multi target hits (#665 @wjian23)
- **core**: Use track_id track WidgetId, which may changed when created by pipe or class. (#665 @wjian23)

## [0.4.0-alpha.15] - 2024-11-13

### Features

- **macros**: Every widget that derives `Declare` will automatically implement a macro with the same name to declare a function widget using it as the root widget. (#651 @M-Adoo)
- **core**: Added the smooth widgets for transitioning the layout position and size. (#645 @M-Adoo)
- **widgets**: Added three widgets `FractionallyWidthBox`, `FractionallyHeightBox`, and `FractionallySizedBox` to enable fractional sizing of widgets. (#647 @M-Adoo)
- **widgets**: Added widget of radio button (#649 @wjian23)
- **core**: `BuildCtx::get()` and `BuildCtx::get_mut()` have been added to facilitate access from anywhere within a build context. (#650 @M-Adoo)

### Fixed

- **core**: The `Provider` might be missing in a pipe class. (#648 @M-Adoo)
- **core**: The child generated by the class may not be mounted. (#648 @M-Adoo)
- **painter**: Scaling the painter to zero resulted in a crash. (#659 @M-Adoo)
- **widgets**: Changing the `flex` of `Expanded` does not trigger a relayout. (#652 @M-Adoo)

### Breaking

- **core**: `Expanded` and `KeyWidget` are not declared with `FatObj`, so they do not currently support built-in widgets. (#648 @M-Adoo)
- **core**: `DeclareObj::finish` does not accept a `BuildCtx` parameter. (#650 @M-Adoo)
- **core**: function widget no longer requires a `&mut BuildCtx` parameter. (#650 @M-Adoo)
- **macros**: Removed the `ctx!` macro. (#650 @M-Adoo)

## [0.4.0-alpha.14] - 2024-10-30

### Features

- **macros**: Added the `part_writer!` macro to generate a partial writer from a mutable reference of a writer. (#642 @M-Adoo)

### Fixed

- **core**: Setting the theme before running the app results in the tree being constructed twice. (#637, @M-Adoo)
- **core**: Resolve a crash occurring in a class implementation with multiple children. (#637 @M-Adoo)
- **core**: Nodes created by a class implementation may not be disposed of when switching to another class. (#637 @M-Adoo)
- **core**: When merge multiple `MixBuiltin` widgets, there may be a premature dropping of the outer `MixBuiltin` before it should occur. (#639 @M-Adoo)
- **core**: `watch!` does not notify the initial value. (#640 @M-Adoo)
- **core**: fix watch multi builtin events not work (#641 @wjian23)
- **core**: fix widget layout when h_algin and v_align are embedded in each other (#641 @wjian23)
- **painter**: fix elements may not be painted after window resize. (#644 @M-Adoo)

### Breaking

- **macros**: Using expression parent (`@(w) { ... }` before `@ $w { ...}`) will no longer automatically wrap a `FatObj` for `w`. (#639 @M-Adoo)

## [0.4.0-alpha.12] - 2024-10-09

### Features

- **core**: Added the built-in widget `TextStyleWidget`, allowing any widget to easily configure the text style within it using `text_style`. (#635, @M-Adoo)

### Breaking

- **text**: Removed the `ribir_text` crate and integrated it into the `ribir_painter` crate. (#635 @M-Adoo)

## [0.4.0-alpha.11] - 2024-10-02

### Features

- **core**: Added the `PaintingStyleWidget` built-in widget, enabling any widget to utilize `painting_style` to specify how shapes and paths should be painted within its descendants. (#633 @M-Adoo)

### Changed

- **text**: Merge the `overflow` field to the `TexStyle` structure. (#629 @M-Adoo)

### Breaking

- **text**: Enhance the typography APIs by eliminating `FontSize`, `Pixel`, and `Em`, and directly utilize only logical pixels represented by `f32`.  (#629 @M-Adoo)

## [0.4.0-alpha.10] - 2024-09-25

### Features

- **core**: Added `WrapRender` for a render widget that combines with its child as a single widget tree node. (#626 @M-Adoo)
- **core**: Added `StateWriter::into_render` to covert writer to reader if no other writer exist. (#626 @M-Adoo)
- **core**: Added the built-in widget `Foreground`, enabling any widget to directly utilize `foreground` for configuring the painter brush. (#628, @M-Adoo)
- **painter**: Distinguishes between fill and stroke brushes, allowing the painter to have two default brushes. (#628, @M-Adoo)

### Changed

- **core**: Merged boolean status widgets into a single widget `MixFlags`, including `HasFocus`, `MouseHover` and `PointerPressed`. (#627 @M-Adoo)
- **core**: Reimplemented `HAlignWidget`, `VAlignWidget`, `RelativeAnchor`, `BoxDecoration`, `ConstrainedBox`, `IgnorePoint`, `Opacity`, `Padding`, `TransformWidget`, and `VisibilityRender` as `WrapRender`. (#626 @M-Adoo)

### Fixed

- **core**: The `SplitWriter` and `MapWriter` of the render widget may not be flagged as dirty. (#626, @M-Adoo)

### Breaking

- **painter**: Removed `Painter::brush` and `Painter::set_brush`, now using `fill_brush`, `stroke_brush`, `set_fill_brush`, and `set_stroke_brush` methods instead. (#628 @M-Adoo)

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
  let state = Stateful::value(0132);
  providers!{
    providers: [Provider::reader(state)],
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
          let v = Provider::state_of::<Stateful<i32>>(BuildCtx::get());
          let v = v.unwrap().clone_writer();
          pipe!($v.to_string())
        }
      }
    }
  }
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

- **core**: Introduced `IntoWidget`. (@M-Adoo #612)

  The `IntoWidget` trait allows for the conversion of any widget to the type `Widget`.

### Fixed

**core**: The generation of a pipe widget from another pipe widget may potentially result in a crash. (#612, @M-Adoo)

### Changed

- **core**: Lazy build the widget tree. (#612, @M-Adoo)

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

- **core**: The split functions in `StateReader::part_reader`, `StateWriter::map_writer`, and `StateWriter::split_writer` no longer need to return a reference. (\#568 @M-Adoo)
- **core**: Introduced `StateWatcher` for watching state modifies, which was previously the responsibility of `StateReader`. This results in a cleaner and more compact `StateReader` implementation. (\#556, @M-Adoo)
- **gpu**: Introduced `GPUBackendImpl::max_textures_per_draw` to set a limit on textures per draw phase (\#562 @M-Adoo)
- **gpu**: Updated the `wgpu` implementation of the GPU backend to support WebGL. (\#578, @M-Adoo)
- **ci**: Added wasm test (\#583 @wjian23)
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

- **macros**: polish the compile error message of invalid filed in `@(var) {}` (\#556 @M-Adoo)
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

- **introduction**: added `introduction.md` to introduce Ribir and why choose it.
- **get started**: added the `get_started` series of tutorials to help users get started with Ribir.

<!-- next-url -->
[@Unreleased]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.53...HEAD
[0.4.0-alpha.53]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.52...ribir-v0.4.0-alpha.53
[0.4.0-alpha.52]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.51...ribir-v0.4.0-alpha.52
[0.4.0-alpha.51]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.50...ribir-v0.4.0-alpha.51
[0.4.0-alpha.50]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.49...ribir-v0.4.0-alpha.50
[0.4.0-alpha.49]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.48...ribir-v0.4.0-alpha.49
[0.4.0-alpha.48]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.47...ribir-v0.4.0-alpha.48
[0.4.0-alpha.47]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.46...ribir-v0.4.0-alpha.47
[0.4.0-alpha.46]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.45...ribir-v0.4.0-alpha.46
[0.4.0-alpha.45]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.44...ribir-v0.4.0-alpha.45
[0.4.0-alpha.44]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.40...ribir-v0.4.0-alpha.44
[0.4.0-alpha.40]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.39...ribir-v0.4.0-alpha.40
[0.4.0-alpha.39]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.38...ribir-v0.4.0-alpha.39
[0.4.0-alpha.38]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.37...ribir-v0.4.0-alpha.38
[0.4.0-alpha.37]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.36...ribir-v0.4.0-alpha.37
[0.4.0-alpha.36]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.35...ribir-v0.4.0-alpha.36
[0.4.0-alpha.35]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.34...ribir-v0.4.0-alpha.35
[0.4.0-alpha.34]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.33...ribir-v0.4.0-alpha.34
[0.4.0-alpha.33]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.32...ribir-v0.4.0-alpha.33
[0.4.0-alpha.32]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.31...ribir-v0.4.0-alpha.32
[0.4.0-alpha.31]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.30...ribir-v0.4.0-alpha.31
[0.4.0-alpha.30]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.29...ribir-v0.4.0-alpha.30
[0.4.0-alpha.29]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.27...ribir-v0.4.0-alpha.29
[0.4.0-alpha.27]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.26...ribir-v0.4.0-alpha.27
[0.4.0-alpha.26]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.25...ribir-v0.4.0-alpha.26
[0.4.0-alpha.25]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.24...ribir-v0.4.0-alpha.25
[0.4.0-alpha.24]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.23...ribir-v0.4.0-alpha.24
[0.4.0-alpha.23]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.22...ribir-v0.4.0-alpha.23
[0.4.0-alpha.22]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.21...ribir-v0.4.0-alpha.22
[0.4.0-alpha.21]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.20...ribir-v0.4.0-alpha.21
[0.4.0-alpha.20]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.19...ribir-v0.4.0-alpha.20
[0.4.0-alpha.19]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.18...ribir-v0.4.0-alpha.19
[0.4.0-alpha.18]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.17...ribir-v0.4.0-alpha.18
[0.4.0-alpha.17]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.15...ribir-v0.4.0-alpha.17
[0.4.0-alpha.15]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.14...ribir-v0.4.0-alpha.15
[0.4.0-alpha.14]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.12...ribir-v0.4.0-alpha.14
[0.4.0-alpha.12]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.11...ribir-v0.4.0-alpha.12
[0.4.0-alpha.11]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.10...ribir-v0.4.0-alpha.11
[0.4.0-alpha.10]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.9...ribir-v0.4.0-alpha.10
[0.4.0-alpha.9]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.8...ribir-v0.4.0-alpha.9
[0.4.0-alpha.8]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.7...ribir-v0.4.0-alpha.8
[0.4.0-alpha.7]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.6...ribir-v0.4.0-alpha.7
[0.4.0-alpha.6]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.5...ribir-v0.4.0-alpha.6
[0.4.0-alpha.5]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.4...ribir-v0.4.0-alpha.5
[0.4.0-alpha.4]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.3...ribir-v0.4.0-alpha.4
[0.4.0-alpha.3]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.2...ribir-v0.4.0-alpha.3
[0.4.0-alpha.2]: https://github.com/RibirX/Ribir/compare/ribir-v0.4.0-alpha.1...ribir-v0.4.0-alpha.2
