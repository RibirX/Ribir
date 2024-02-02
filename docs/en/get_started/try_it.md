---
sidebar_position: 1
---

# Try Ribir

> You will learn
>
> - How to write and start a simple `Hello world!` application


## Install Rust

At first, you need to install Rust, you can reference the [official documentation](https://www.rust-lang.org/tools/install).

> Tips
>
> Currently Ribir only supports Rust nightly channel. You can use `rustup override set nightly` to switch to nightly channel. If you haven't installed nightly channel yet, you can check out [rustup Channels documentation](https://rust-lang.github.io/rustup/concepts/channels.html).

## Create a new Ribir project

Then, open your terminal and create a new Rust project:

```sh
cargo new ribir-hello-world
cd ribir-hello-world
```

Next, edit the `Cargo.toml` file and add Ribir as a dependency:

```toml
[dependencies]
ribir = "@RIBIR_VERSION"
```

Or you can directly run `cargo add --git "https://github.com/RibirX/Ribir" ribir` to let Cargo add the latest Ribir version that is under development for you.

## Start writing `main.rs`

Open your editor and modify the `src/main.rs` file to:

```rust
// main.rs
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! { @Text { text: "Hello World!" }});
}
```

## Run the application

```sh
cargo run
```

Congratulations! You have completed your first Ribir project.

## Run Ribir's built-in examples

Finally, there are some other examples in the Ribir repository, you can clone the Git repository:

```sh
git clone git@github.com:RibirX/Ribir.git
cd Ribir/Ribir
```

and run the examples with one of the following commands:

```sh
cargo run -p counter
cargo run -p storybook
cargo run -p messages
cargo run -p todos
cargo run -p worle_game
```