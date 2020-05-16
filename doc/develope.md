# Develop Guide


## Test

We mainly use the official test harness to test and bench our project. But some test case we need control in a custom way. 

### How to write your unit test ?

Usually just follow rust official test guide, except two  

1. Your test case need gpu support, for example, need create a canvas. In generally these cases are not available in ci environment. So we need point out ignore it, otherwise ci may failure. So, just add a `ignore` attr for it, like below.

```rust
  #[test]
  #[ignore = "gpu need"]
  fn canvas_draw_circle() {
    let mut canvas = block_on(Canvas::new(DeviceSize::new(400, 400)));
    // ...
  }
```

`#[ignore = "gpu need"]`, that all.


2. Your test must run in main thread, for example it's depend on a event loop etc.

To ensure test run in main thread, we need to disable official test harness in `cargo.toml`. And provide its own main function to handle running tests. 

```
[[test]]
name = "canvas"
path = "main_thread_tests/canvas.test.rs"
harness = false
```

Now, the `main` function in `main_thread_tests/canvas.test.rs` can testable. But the output is not friendly, and whole file assume as one unit test.

We provide a crate `unit_test`  to help to pack many tests, so you can write many tests in the file and output friendly, like this:


```rust
// main_thread_tests/canvas.test.rs

fn test_first() {
  // write your first test here
}

fn test_second {
  // write your second test here
}

fn main() {
  unit_test_describe!{
    run_unit_test(test_first);
    run_unit_test(test_second);
  }
}

```

## How to run tests ? 

In develop environment, you should run all test:

```
cargo test --  --include-ignored -Z unstable-options
```

In the ci environment we use two command to run different tests.

First, and the major, test the tests by official harness and not require gpu support:

```
cargo test
```

Second, we run only the tests be ignored that may access gpu ability. Also required the ci environment provide gpu support:

```
cargo test -- --ignored 
```