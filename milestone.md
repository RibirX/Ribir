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
  - [ ] gpu environment support [Test](./doc/develope.md#Test)
- [ ] perform layout on render tree (2 week)
  - [x] layout flow
  - [x] base layout widget
    - [x] Row
    - [x] Column
    - [ ] Center

## paint & event, runnable (6.1)

- [x] Widget Derive
- [x] paint
  - [x] which 2d graphic library to use?
  - [x] paint flow.
- [x] Theme data.
- [ ] event  
  - [ ] window event
  - [x] event loop
  - [x] bubbling framework
  - [ ] event type
    - [x] point event ?
    - [ ] window event ?
    - [x] keyboard event
      [x] wheel event

- widgets for todo demo
- [ ] simple widget
  - [ ] Button
  - [ ] Text
  - [ ] List
  - [x] Scrollable
  - [ ] Table scroll
  - [x] Checkbox
- [ ] layout widget
  - [ ] Center
  - [x] Row
- [ ] event widget
  - [ ] input widget.
  - [x] click event listener

- [x] A derive macro for state impl
- [x] refactor stateful
  - [x] stateful should be a widget not a attr.
  - [x] into_sateful should not depend on `BuildCtx`
- [x] auto implement declare macro to build widget.

##  we need readable & learnable documents.

- [ ] readme
- [ ] contributing
- [ ] tutorial
- [ ] api docs cover
- [ ] inside framework

## cross platform

- [x] osx
- [x] linux, already add a test in ci.
- [ ] windows
- [ ] android
- [ ] ios
- [ ] web / WebAssembly


## animation

## parallelism layout 

## multi main window

## infinite / virtual scroll

## debug, test and productive develop tools

## declare language to describe ui.

provide `declare!` macro.

