# Quick Start

## Prerequisites

The first step is to install Rust, and others system dependencies.

### Installing

You can reference [official documentation](https://www.rust-lang.org/tools/install).

### System dependencies

if you're Ubuntu/Debian:

```shell
sudo apt install libx11-dev cmake pkg-config libfreetype6-dev libexpat1-dev libfontconfig1-dev
```

## Setting up Ribir

The first you need create a new Rust Application.

```shell
cargo new ribir-hello-world
cd ribir-hello-world
```

> Tips
> 
> Ribir only support rust nightly version now. You can use `rustup override set nightly` to switch channel. If you don't install nightly version, you can reference this [documentation](https://rust-lang.github.io/rustup/concepts/channels.html).

And next, edit the `Cargo.toml` file and add Ribir dependencies:

```toml
[dependencies]
ribir = "0.0.1"
```

## Starting with `main.rs`

Open `main.rs` file by your editor and modify it like this:

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
