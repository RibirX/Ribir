# Dev helper

This library provides macros to write tests for `Ribir`.

## Dependencies
 
To use these macros add `paste` and `ribir_dev_helper` in `[dev-dependencies]` section of your `Cargo.toml`. For the detail see the macros documents.

## Test case files

These macros may read files to test, all those files are read from the `test_cases` in the workspace root of your project. 

Use `RIBIR_IMG_TEST=overwrite` environment variant to overwrite or generate the files. For example, you can use `RIBIR_IMG_TEST=overwrite cargo test` to overwrite all test case files. Or special test `RIBIR_IMG_TEST=overwrite cargo test -- test_name`.