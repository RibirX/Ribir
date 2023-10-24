# Ribir Examples

All the examples in this folder use the macro `example_framework!` to startup, the examples will generate tests and benchmarks for the example to ensure every modification work for those examples.

Run examples:

``` sh
cargo run -p storybook --features="wgpu"
```

Remember add `--features="wgpu"` to use `wgpu` painter-backend to render, we not enable it as default.