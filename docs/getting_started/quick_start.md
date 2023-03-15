---
sidebar_position: 1
---

# Quick Start

## Prerequisites

The first step is to install Rust, and others system dependencies.  
You can reference the [official documentation](https://www.rust-lang.org/tools/install).

## Setting up Ribir

Then create a new Rust Application:

```shell
cargo new ribir-hello-world
cd ribir-hello-world
```

> Tips
> 
> Currently Ribir only support Rust nightly version. You can use `rustup override set nightly` to switch to the nightly channel. If you do not have the nightly channel installed yet, the [rustup Channels documentation](https://rust-lang.github.io/rustup/concepts/channels.html) explains how to get it.

Next, edit the `Cargo.toml` file and add Ribir as dependency:

```toml
[dependencies]
ribir = "0.0.1"
```
or just run `cargo add --git "https://github.com/RibirX/Ribir" ribir` to let Cargo do it for you.


## Starting with `main.rs`

Open the `src/main.rs` file in your editor and change it to:

```rust
// main.rs
use ribir::prelude::*;

fn main() {
  let hello_world = widget! {
    Text { text: "Hello World" }
  };

  app::run(hello_world);
}
```

## Run Application

```shell
cargo run
```

[todo: hello world demo show placeholder]

Congratulations! You finish the first Ribir project.

## Run Examples

To run the examples included with Ribir check out the Git repository

```sh
git clone git@github.com:RibirX/Ribir.git
cd Ribir/ribir
```

and run the examples with

```rust
cargo run --example animation_demo                   
cargo run --example counter                          
cargo run --example greet                            
cargo run --example todo_mvp                
```
