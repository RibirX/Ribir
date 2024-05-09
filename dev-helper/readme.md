# Development Helper

This library offers macros to facilitate testing for `Ribir`.

## Dependencies

To utilize these macros, include `paste` and `ribir_dev_helper` in the `[dev-dependencies]` section of your `Cargo.toml`. For more details, refer to the macro documentation.

## Test Case Files

These macros may require reading files for testing. All such files are sourced from the `test_cases` directory located at the root of your workspace.

Use the `RIBIR_IMG_TEST=overwrite` environment variable to overwrite or generate the files. For instance, `RIBIR_IMG_TEST=overwrite cargo test` can be used to overwrite all test case files. For a specific test, use `RIBIR_IMG_TEST=overwrite cargo test -- test_name`.

For image tests, if the actual image differs from the expected one, both the actual image and the difference image are saved alongside the expected image. The difference image represents the discrepancies between the actual and expected images.