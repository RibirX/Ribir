# Core principles

## Phase
                    Main thread                       |             Parallelism   
                        .                             |                   
      Build Phase     --.-->  Render Ready Phase    --|-->    Layout    --|--> Paint
                        .                             |                   |
  inflate widget tree   .   construct render tree and |                   |
  or rebuild subtree    . update render tree data from|                   |
                        .         widget tree         |                   | 
                      <------------------ May back build phase again <----|

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

a. build widget from `CombinationWidget`.
b. if new widget 's `Key` is equal to the last time build widget in the widget tree ?
  1. use new widget replace before sub tree in widget and mark this widget dirty.
  2. if this widget is `CombinationWidget`, use new widget recursive step a.
  3. else, if this widget is render widget and has children.
    * pluck all children from widget tree.
    * process new children one by one
      - if a old child can be found by new child's key in plucked children.
        * insert old child back.
        * recursive step 1.
      - else add new widget in widget tree, and recursive step c.
      - destroy the remaining plucked child and subtree, correspond render tree destroy too.
  4. else done.
c. else, inflate the new widget and use the new widget subtree instead of the old in widget tree, reconstruct render subtree correspond to this widget subtree.
d. done, the subtree from this widget is rebuild finished.



### Signature ?

`Key` is used for widget, and `Signature` used for render object. Not like `Key` to identify if this widget is a same widget with before. `Signature` guarantee that if two same type render objects have same `Signature` that mean the have same content and will paint same thing. So `Holiday` can treat all render objects as one, if they have same type and same `Signature`.


## Compose prefer

Not like many classic GUI framework, `Holiday` not built on inherit mode. Because Rust not support inherit, `Holiday` built base on composition. For a example, If you want give a `Button` widget opacity, it's not have a field named `opacity` give you to set the opacity, you should use a `Opacity` widget to do it, like:

```rust
Opacity {
  opacity: 0.5,
  Button! { text: "Click Me!"}
}
```

### zero cost for compose ?


## RenderObject & RenderTree 

Widget tree use to represents the user interface, and RenderTree is created by RenderWidget do the actually layout and paint. In this vision, widget is build cheap, and RenderObject is more expensive than widget.

## Update Data

When we build a widget from CombinationWidget, framework provide a BuildContext param, and the NodeId of the widget build in the ctx, use the NodeId framework can provide State<Self> to change widget state when reactive user interactive. 

### Layout

`Holiday` performs one layout per frame, it's a recursive layout from parent down to children. There is some point to help write a layout:
1. the layout has some constraints:
    1. DECIDED_BY_SELF which mean it's size layout can just deceded by self, it's conflict to the other constraint.
    2. EFFECTED_BY_PARENT which mean it's size layout will be affected by it's parent, so when parent need reperform layout it should be relayout too.
    3. EFFECTED_BY_CHILDREN which mean it's size layout will be affected by it's children, so when it's children need reperform layout it should be relayout too.

2. all the render object is independent, all owned by render tree. in the processing of performing layout, it can only accept immutable reference of render object  of it's parent or it's children through NodeId of itself and render ctx.

3. when mark_need_dirty of a render object, it will start from the current Node, climbing up until it's parent's constraint has no flag of EFFECTED_BY_CHILDREN, the  ancestor in the path will be mark dirty and diffuse dirty flag down to children who has constraint flag of EFFECTED_BY_PARENT recursive.



## Children Relationship

In `Holiday`, normal render object not hold the children, but we can use layout widget to build a parent & children relationship.


## avoid to rebuild widget ?