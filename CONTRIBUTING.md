# Contributing to Ribir

Thank you for your interest in contributing to Ribir! We welcome bug reports, feature requests, and pull requests from everyone.

Please first discuss the change you wish to make via an issue before making a major change.

## 🛠 Local Development Tools

To ensure high code quality and consistency with our CI pipeline, we provide a local check script.

### Using the CI Script (`tools/ci.rs`)

`tools/ci.rs` is a Rust-based script (using `cargo-script`) that mirrors our GitHub Actions workflow. You can run it locally to verify your changes.

#### How to run:
*   **Via cargo (requires nightly):** `cargo +nightly ci [command]`
*   **Directly:** `./tools/ci.rs [command]`

#### Common commands:
*   `all`: Run all checks (default).
*   `fmt` (or `f`): Check code formatting.
*   `clippy` (or `c`): Run Clippy lints.
*   `check`: Run `cargo check` (using stable).
*   `lint` (or `l`): Run all lint checks (`fmt` + `clippy` + `check`).
*   `test` (or `t`): Run tests (includes coverage if `cargo-llvm-cov` is installed).
*   `doctest` (or `d`): Run code examples in documentation.
*   `wasm` (or `w`): Verify compilation for `wasm32-unknown-unknown`.
*   `bundle` (or `b`): Verify the bundle process using the counter example.
*   `config`: Show current CI configuration and toolchain versions.

---

## 📸 Visual Testing

Ribir uses pixel-matching tests to ensure UI rendering remains correct.

*   **Test Cases**: Visual assets are stored in the `test_cases/` directory at the project root.
*   **Updating Results**: If you've intentionally changed the rendering and verified it is correct, update the expected images by running:
    ```bash
    RIBIR_IMG_TEST=overwrite cargo test -- [test_name]
    ```
*   **Inspecting Failures**: If a visual test fails, an "actual" image and a "difference" image will be generated in `test_cases/` alongside the expected image for easy comparison.

### Widget Testing

If you're developing a widget, the [`ribir_dev_helper`](https://docs.rs/ribir_dev_helper) crate provides useful macros for testing. See its documentation for details.

---

## 🚀 Pull Request Process

1.  **Format and Lint**: Before committing, please run `./tools/ci.rs lint` to ensure your code matches the project style and passes static analysis.
2.  **Add Tests**: If you're adding a new feature or fixing a bug, please include corresponding tests.
3.  **Update Documentation**: Update `README.md` or relevant files in `docs/` if your change affects the public API or environment variables.
4.  **Versioning**: Prior to version 1.0, we do not strictly adhere to SemVer. For details on our versioning policy and release process, please refer to [RELEASE.md](./RELEASE.md). Version numbers are managed via GitHub Actions.
5.  **Review**: PRs require sign-off from at least one core developer before they can be merged.

---

## 📜 Code of Conduct

### Our Pledge
In the interest of fostering an open and welcoming environment, we as contributors and maintainers pledge to making participation in our project and our community a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Our Standards
Examples of behavior that contributes to creating a positive environment include:
*   Using welcoming and inclusive language
*   Being respectful of differing viewpoints and experiences
*   Gracefully accepting constructive criticism
*   Focusing on what is best for the community
*   Showing empathy towards other community members

### Enforcement
Instances of abusive, harassing, or otherwise unacceptable behavior may be reported by contacting the project team. All complaints will be reviewed and investigated and will result in a response that is deemed necessary and appropriate to the circumstances.
