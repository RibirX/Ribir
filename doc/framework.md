# Core principles

## Declare & Reactive programming mode

`Holiday` build a widget tree by your ui declare for data, and create a render object tree to layout and paint.
When widget depended data changed, this widget tree reactive to a update, and the render objects correspond to the updated widgets will update too. 

When a widget as a root to run in `Application`, `Holiday` inflate it to a widget tree. A widget tree is a tree which every leaf is a render widget. Sometimes, only update tree is not enough, `CombinationWidget` may build a completely difference `Widget` to last. In these scenes, corresponds subtree need to rebuild. Because every widget must explicit provide a rebuild emitter or not by `RebuildEmitter` trait, so the widget tree know how to efficient rebuild a subtree across a widget diff algorithm. `Holiday` will implement a `RebuildEmitter` for every widget, so it's not a burden to implement a widget. Widget also have a chance to implement it by itself, because widget self know which time is really need to rebuild, and how to have the best performance. 

## Rebuild Widget Subtree Diff Algorithm

Widget not update or rebuild subtree immediately when its rebuild emitter emitted. It's just mark this widget need rebuild and wait until the widget tree rebuild. 

Widget tree do updating from top to bottom. If a bottom widget removed because its ancestor rebuild, its update or rebuild auto be canceled. Even if `CombinationWidget` require rebuild, that not mean `Holiday` will reconstruct the total subtree, the `Key` may help us to reduce many cost in some case.

### Key

`Key` help `Holiday` to track which widgets add, remove and changed. So `Holiday` can modify widget tree and render tree minimal. A `Key` should unique for each widget under same father.

Widget tree do rebuilding base on widget diff. Work like below:

1. if this widget is `CombinationWidget`:
  a. build widget from `CombinationWidget`.
  b. if new widget's `Key` is equal to the last time build widget in the widget tree ?
    * only use new widget instead of old widget in the widget tree, and not inflate.
    * use new widget and recursive to step 1.
  c. else, inflate the new widget and use the new widget subtree instead of the old in widget tree.
  d. done, the subtree from this widget is rebuild finished.
2. else, if this widget is render widget without children ?
  a. if this widget has same `Key` as before ?
   * Yes, we need do nothing any more.
  b. else, use new widget replace before sub tree in widget.
3. else, use children recursive to step 1 one by one.. 

### Signature

`Key` is used for widget, and `Signature` used for render object. Not like `Key` to identify if this widget is a same widget with before. `Signature` guarantee that if two same type render objects have same `Signature` that mean the have same content and will paint same thing. So `Holiday` can treat all render objects as one, if they have same type and same `Signature`.


## Compose prefer

Not like many classic GUI framework, `Holiday` not built on inherit mode. Because Rust not support inherit, `Holiday` built base on composition. For a example, If you want give a `Button` widget opacity, it's not have a field named `opacity` give you to set the opacity, you should use a `Opacity` widget to do it, like:

```rust
Opacity! {
  opacity: 0.5,
  Button! { text: "Click Me!"}
}
```

## RenderObject & RenderTree 

Widget tree use states represents the user interface, and RenderTree is created by RenderWidget do the actually layout and paint. In this vision, widget is build cheap, and RenderObject is expensive.

### Layout

`Holiday` performs one layout per frame, it's a recursive layout from parent down to children. There is some point to help write a layout:

There only three type size render object can be:

1. Fixed Size, a constant size gave before layout.
2. Expand Size, auto size defect by itself content or children size.
3. Bound Size, auto size defect by its parent layout.

if a render object is Expand Size, its children can't have Bound Size. In genera, when perform a layout, fixed size children should be first perform, then expand size and bound size last.


## Children Relationship

In `Holiday`, normal render object not hold the children, but we can use layout widget to build a parent & children relationship.


## avoid to rebuild widget ?