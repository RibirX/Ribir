# Develop Guide


## Test

We mainly use the official test harness to test and bench our project. But same test case we need control in a custom way. Usually these two cases we don't use official way:

1. Case must run in main thread, like need create a event loop etc.
2. Case need gpu support, but generally not available in ci environment.

So we split test cases into two parts by feature `main-thread-test`. 

In develop environment, you should run all test:

```
cargo test --all-features
```

In the ci environment we use two command to run different tests.

First, and the major, test the tests by official harness and not require gpu support, neither need run in main thread:

```
cargo test
```

Second, we run only the tests be limited in main thread and access gpu ability. Also required the ci environment provide gpu support:

```
cargo test --all-features main-thread-test 
```

### How to write test with `main-thread-test`

First, disable official test harness in cargo.toml. And a your test path like below, remember add `required-features = ["main-thread-test"]` so your test will run only with feature `main-thread-test`.

```
[[test]]
name = "canvas"
path = "main_thread_tests/canvas.test.rs"
harness = false
required-features = ["main-thread-test"]
```

Now, the `main` function in `main_thread_tests/canvas.test.rs` can testable. But the output may not friendly, and whole file assume as an unit test.

We provide an `unit_test_describe` to help to pack many tests, so you can write many tests in the file and output friendly, like this:


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