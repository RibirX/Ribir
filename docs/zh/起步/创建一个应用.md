# 创建一个 Ribir 应用

这篇文档将向你介绍如何配置和创建一个 Ribir 应用。

> 你将了解
>
> - 如何编写和启动一个简单的 `Hello world!` 应用


## 安装 Rust

首先, 你需要安装 Rust，你可以参考 [Rust 官方文档](https://www.rust-lang.org/tools/install).

> 提示
>
> 目前 Ribir 只支持 Rust nightly 版本。 你可以使用 `rustup override set nightly` 切换到 nightly 通道。如果你还没有安装 nightly 通道，可以查看 [rustup Channels 文档] (https://rust-lang.github.io/rustup/concepts/channels.html)。

## 新建 Ribir 项目

然后，打开你的终端，创建一个新的 Rust 项目:

```sh
cargo new ribir-hello-world
cd ribir-hello-world
```

接下来, 编辑 `Cargo.toml` 文件, 添加 Ribir 作为依赖:

```toml
[dependencies]
ribir = "@RIBIR_VERSION"
```

或者直接运行 `cargo add --git "https://github.com/RibirX/Ribir" ribir` 让 Cargo 为你添加正在开发中的最新 Ribir 版本.

## 编写 `main.rs`

打开编辑器, 将 `src/main.rs` 文件修改为:

```rust
// main.rs
use ribir::prelude::*;

fn main() {
  App::run(fn_widget! { @Text { text: "Hello World!" }});
}
```

## 运行应用

```sh
cargo run
```

恭喜! 你完成了第一个 Ribir 项目。

## 运行 Ribir 自带示例

最后，Ribir 仓库中还有一些其他示例，你可以克隆 Git 仓库:

```sh
git clone git@github.com:RibirX/Ribir.git
cd Ribir/Ribir
```

并使用以下命令之一运行示例:

```sh
cargo run -p counter
cargo run -p storybook
cargo run -p messages
cargo run -p todos
```

