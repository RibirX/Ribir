# Core principles

## Phase
                        Main thread                        |             Parallelism   
                            .                              |                   
      Build Phase         --.-->  Render Ready Phase    ---|-->    Layout    --|--> Paint
                            .                              |                   |
inflate the widget tree     .   construct render tree and  |                   |
  or rebuild subtree        . update render tree data from |                   |
                            .         widget tree          |                   | 
                         <------------------- May back build phase again <-----|

## Declare & Reactive programming mode

`Holiday` builds widget tree with your ui by declare data, meanwhile it creates render object tree to layout and paint.
When the data that the widget depends on changes, widget tree will make a update, and render objects correspond updated widgets to update also.

When a widget as root node run in `Application`, it will be inflated into widget tree by framework. Every leaf in the widget tree is a rendered widget. Sometimes, only has tree updated is not enough, it's possible that `CombinationWidget` builds a complete full differently `Widget`.  So if a `CombinationWidget` is changed, it need rebuild and maybe reconstruct full subtree. Framework try to rebuild the widget tree and the render tree as mini as possible.

## Rebuild Widget Subtree Diff Algorithm

Widget doesn't update or rebuild subtree immediately when its state changed. It's just mark this widget need to rebuild and wait until the widget tree rebuild. 

The widget tree update from top to bottom. If a bottom widget removed because its ancestor rebuild, its update or rebuild auto be canceled. Even if `CombinationWidget` require rebuild, itself must be rebuild, but that not mean `Holiday` will reconstruct the total subtree, the `Key`may help us to reduce many cost in some case.

### Key

`Key` helps `Holiday` to track what widgets add, remove and changed. So `Holiday` can modify the widget tree and the render tree minimally. A `Key` should unique for each widget under the same father.

The widget tree rebuilds base on widget diff. Work like below:

a. build widget from `CombinationWidget`.
b. if the `key` of widget is equal to the last time build widget in the widget tree ?
  1. use new widget replace before sub tree in widget and mark this widget dirty.
  2. if this widget is `CombinationWidget`, use new widget recursive step a.
  3. else, if this widget is render widget and has children.
    * pluck all children from widget tree.
    * process new children one by one
      - if a old child can be found by new child's key in plucked children.
        * insert old child back.
        * recursive step 1.
      - else add new widget in the widget tree, and recursive step c.
      - destroy the remaining plucked child and subtree, correspond render tree destroy too.
  4. else done.
c. else, inflate the new widget and use the new widget subtree instead of the old in widget tree, reconstruct render subtree correspond to this widget subtree.
d. done, the subtree from this widget is rebuild finished.



## Compose prefer

Unlike many classic GUI framework, `Holiday` doesn't build on inherit mode. Because Rust not support inherit, `Holiday` built base on composition. For example, If you want give a `Button` widget opacity, it doesn't have a field named `opacity` give you to set the opacity, you should use a `Opacity` widget to do it, like:

```rust
Opacity {
  opacity: 0.5,
  Button! { text: "Click Me!"}
}
```

### Widget Attribute


## RenderObject & RenderTree 

The widget tree corresponds to the user interface, and `RenderTree` is created by RenderWidget, it do the actually layout and paint. In this vision, widget is build cheap, and RenderObject is more expensive than widget.

## Stateless and Stateful

As default, every widget is stateless, just present like what you declare and no interactive. But in real world we often need change widget to another state to respond to user actions, IO request and so on. A way to support it is rebuild the whole widget tree and do a tree diff to update the minimal render tree. But we provide another way to do it, every widget can across `into_stateful` convert to a stateful widget, a `StateRef` will also return at the same time, which can be used to modify the states of the widget.

### Layout

`Holiday` performs a layout per frame, and the layout algorithm works in a single pass. It's a recursive layout from parent down to children. 

There is some important point to help understand how to write a layout:

1. RenderObject not directly hold its children, and have no way to directly access them.
2. `RenderTree` store the `RenderObject`'s layout result, so `RenderObject` only need to provide its layout algorithm in `perform_layout` method.  The `perform_layout` have two input, a `BoxClamp` that limit the min and max size it can, a `RenderCtx` provide the ability to call the `perform_layout` of its children, and it a way to know the size the child need.
3. `RenderObject::perform_layout` responsible for calling every children's perform_layout across the `RenderCtx`。
4. The `BoxClamp` it always gave by parent.

`only_sized_by_parent` method can help framework know if the `RenderObject` is the only input to detect the size, and children size not affect its size.

## Children Relationship

In `Holiday`, normal render object not hold the children, but we can use layout widget to build a parent & children relationship.


## avoid to rebuild widget ?