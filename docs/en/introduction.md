---
sidebar_position: 1
---

# Introduction

> In this chapter, we will use the Ribir syntax to write some simple examples. You only need to understand the general idea, we'll go into more detail in the following chapters.

## What is Ribir?

Ribir is an open-source Rust framework for building beautiful, native, multi-platform applications from a single codebase.

Ribir uses a non-intrusive declarative programming model that allows you to develop and design user interfaces as an independent module.

Its core design concept is:

> The UI is a re-description of data interaction and continues to respond to modifications of data.

The "re" here signifies that the API serves as the initial description of the data interaction. When building with Ribir, developers only need to focus on the data API to create the UI.

## Why choose Ribir?

### Non-intrusive programming model

Ribir only interacts with the API of your data and does not require any pre-design of your data for the user interface: 

- No additional states
- No additional notification mechanisms 
- No inheritance of any base classes
- No other pre-constraints. 

It doesn't break the logic and structure of your existing data or inject any additional objects. When developing the core part of an application, you can focus on designing the data, logic, and API of the application without thinking about the UI at all.

The UI directly operates data, and data modifications directly drive UI updates, without any intermediate layers and concepts.

### Consistent experience across multiple platforms, and easy to expand to new platforms

Ribir can be used to develop desktop, mobile, web and server-side rendering applications. It generates efficient binary code or WASM programs without relying on any runtime environment. It outputs a very simple, platform-independent drawing result, allowing you to choose to be rendered entirely by the GPU or CPU. You can even easily implement your own rendering backend to expand to uncovered platforms.

### The Declarative syntax that is easy to interact with Rust

Ribir provides a declarative syntax that is easy to interact with Rust. It is not a new language, but a set of Rust macros. Therefore, it can interact well with Rust, making your code both a clear view description and a powerful logical expression, without any environment and tool dependencies.

### Point-to-point view update strategy

Ribir will map a view tree based on your description of the data, and the view will be updated in response to data modifications - this update does not rebuild the entire view, but updates the parts of the view that depend on the modified data point-to-point.

The update logic is determined at compile time, and there is no general diff or patch algorithm to execute at runtime.


### "Pay-as-you-go" design principle

Due to the need to handle multiple complex real-life situations, a general GUI framework tends to have complex designs and extensive features. As a result, it is difficult to keep it lightweight. The way Ribir balances this problem is to provide enough capabilities to ensure development efficiency, and requiring that all capabilities only need to be understood and have overhead when they are used. A few examples:

**Pure composition**: Ribir uses widgets to build interfaces. Unlike common object-oriented GUI frameworks, Ribir widgets do not need to inherit a base class or hold a base object. It is a pure composition model, even the parent-child relationship and built-in fields are completed through composition. The advantage of this is that the widget only needs to focus on the capabilities it provides, so it can be made very small to improve reuse. For example, Ribir has many very mini built-in widgets, and using these built-in widgets to extend ordinary widgets is powerful, but does not bring any overhead to them. For example:

```rust
use ribir::prelude::*;

fn_widget!{
  @Text {
    // `margin` is not a field of `Text`,
    // it is a field of the built-in widget `Margin`,
    // but it can still be used directly by `Text`.
    margin: EdgeInsets::all(8.),
    text: "Hello world!"
  }
};
```

The above example shows the way of combining built-in widgets. Even if `Text` does not have a `margin` field, you can still use the `Margin::margin` and compose it with `Text` to form a new widget. `Margin` will only be created when a widget uses the `margin` field, otherwise, there will be no overhead.

**Digestion of compose widget**: When describing the view of the data, in addition to some basic widgets, most widgets are composed of other widgets. For instance, a `Button` is an assembly of elements like `Text`, `Icon`, and `BoxDecoration`. The `Button` itself isn't a visual component; we refer to such a widget as a compose widget. Compose widgets will be digested during view construction. They are similar to a function that is invoked once during view construction and do not exist in the final view.

**Sateful without writing source will convert to Stateless**: Unlike other declarative frameworks that add fields to widgets to control widget updates. Ribir is non-intrusive. Ribir treats the entire widget as a state to control updates. It provides the ability to split the state so that the local view can directly depend on the modification of part of the data to update (introduced in detail in the subsequent tutorial). Another big difference is that stateful and stateless can be converted to each other. If a state has no write source, it will degenerate into stateless， because no one will update it. For example:

```rust
use ribir::prelude::*;

fn_widget!{
  let show_hi = Stateful::new(true);
  @Text {
    visible: pipe!(*$show_hi),
    text: "Hello world!"
  }
};
```

In the above example, we declared a `Text` and used the `pipe!` macro to directly associate the visibility of `Text` with `show_hi`. But this association will be removed when the view is constructed because `show_hi` has no written source. Therefore, Ribir constructs a simple static view.

### Reliability

Unlike general GUI frameworks that use inheritance and do not have any type constraints except base class inheritance, Ribir builds views based on widget composition and relies on the types between parent and child widgets to constrain whether and how to compose them. You can standardize your child types, so many errors can be reported at compile time instead of being checked at runtime.


## What is the current status of Ribir?

### Stability

The core framework of Ribir is in a stable state, and the API and syntax will be iterated with a cautious attitude. Although the widget library already has many available widgets, it is still in a very rough state, and each version will have major changes.

### Platform coverage

The 0.1 version only covers the Mac, Linux and Windows platforms. You can try to compile the project to the corresponding mobile and web ends, but they have not been verified.

### Performance

In all important designs of the entire framework, performance is an important factor we consider. According to the performance of real development projects we have observed, the overall experience meets expectations. We expect it to eventually have excellent performance. But to be honest, we have never done any detailed performance measurement and analysis, so we have not done any code optimization work. We expect this work to be carried out comprehensively after the full platform coverage and the widget library are relatively stable - or we have encountered detailed performance bottlenecks before that.

### Who is using Ribir?

**Polestar Chat**: 

**Sisyphus**: An editor for editing interactive documents, this is a long-term project, that is still in the early design and development stage, and it is the idea of this project that led to the birth of Ribir.
