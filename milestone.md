# Milestone

## core concept, tree framework (5.1)
 
- [x] widget tree
  - [x] a render widget to test render tree?
  - [x] a combination widget to test widget tree?
- [x] render tree
- [x] rebuild sub tree (1 week)
- [x] react widget change (1 week)
- [ ] ci & workflow
  - [x] mergify bot
  - [x] unit test 
  - [x] code cover
  - [ ] benchmark comparison [#2](https://github.com/M-Adoo/Ribir/issues/2)
  - [x] merge framework branch to master.
  - [ ] (lavapipe?) gpu environment support [Test](./doc/develope.md#Test)
- [x] perform layout on render tree (2 weeks)
  - [x] layout flow
  - [x] base layout widget
    - [x] Row
    - [x] Column
    - [x] Center
    - [x] Flex

## TodoMVP

- [x] Widget Derive
- [x] paint
  - [x] which 2d graphic library to use?
  - [x] paint flow.
- [x] Theme data.
- [ ] event  
  - [ ] window event
  - [ ] application event, block on multi main window support
  - [x] event loop
  - [x] bubbling framework
  - [x] common events
    - [x] point event ?
    - [x] keyboard event
    - [x] wheel event
    - [x] focus event

- [x] include_svg.
- widgets for todo demo
- [ ] base widget
  - [ ] Button
  - [x] Text
  - [ ] ListView
  - [x] Scrollable
  - [x] Checkbox
- [ ] input widget.
- [ ] animations
- [x] A derive macro for state impl
- [x] refactor stateful
  - [x] into_sateful should not depend on `BuildCtx`
- [x] auto implement declare macro to build widget.

-## we need readable & learnable documents.
  - [ ] readme
  - [ ] contributing
  - [ ] tutorial
  - [ ] api docs cover
  - [ ] how framework work

- [x] compose trait accept stateless/stateful enum so we not always convert a widget to stateful.

## cross platform

- [x] osx
- [x] linux, already add a test in ci.
- [x] windows
- [ ] android
- [ ] ios
- [ ] web / WebAssembly


##  keep type information to optimize tree.

For now, we have `Widget` as an abstract node to hold everything user declare, and then construct a tree from it. When user declare tree convert to `Widget` information have be erased. 

if we omit this pre box and build widget tree directly one by one, so we can keep all type information when build the tree, and can do many optimistic across the type information.

- all compose widget have be consumed.
- stateless parent and son render widget can be merged until not only sized by parent.
- some stateless render sibling can be merged.



## parallelism layout 

## multi main window

## infinite / virtual scroll

## debug, test and productive develop tools

## declare language to describe ui.

provided `declare!` macro.


## zero cost compose widget

a. compose widget can support directly update its subtree by compile time analyze
  - block by d
b. compose widget not exist in widget tree
  - block by d
c. compose widget should be concrete type
  - after c & b finished, compose widget should be zero cost.
  - block by b & i.
d. we can remove `Attribute` concept and use compose widget to implement it.
  - compose widget can be zero cost after b & c
  - block by i.
e. we can use compose widget to implement animate, should it not depends on `BuildCtx` in dsl
f. we can use compose widget as DeclareBuilder so it can not depends on `BuildCtx` in dsl
g. `widget!` macro should not depends on `BuildCtx`
  - block by e & f
h. `widget!` macro can use anywhere, and not depends on any context.
  - should add a reactive blocks to track outside widget change.
  - block by e & f & g
  - remove `#[widget]` attr
i. add `QueryType` trait to find type information 

