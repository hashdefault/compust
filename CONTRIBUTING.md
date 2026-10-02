# Contributing to Compust

[English (US)](CONTRIBUTING.md) | [Português (Brasil)](CONTRIBUTING.pt-BR.md)

Compust is early enough that a clear bug report, a driver test, or a small documented improvement can shape the project. Issues and pull requests may be written in English or Brazilian Portuguese. Be specific, respectful, and explain what you observed.

## Set up a development environment

Install rustup, a C linker, Git, and Xvfb. The repository pins Rust 1.95.0. Fork the repository, clone your fork, and create a branch for one change. Keep your normal desktop compositor running while developing against a nested server.

```sh
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```

`cargo test` starts isolated Xvfb instances with dynamically allocated displays, launches the actual binary, and checks pixels and X11 state. It needs neither your current `DISPLAY` nor a running desktop. Override the executable with `XVFB=/path/to/Xvfb`. The unit tests alone can be run with `cargo test --bin compust`.

The tests wait for X11 events with deadlines. Preserve that approach: arbitrary delays make graphics tests flaky. Animation tests can observe time because animation is the behavior under test. Avoid assertions about log wording or private implementation details when the output pixels or protocol state can be checked.

## Choose useful work

The [roadmap](docs/ROADMAP.md) separates correctness, performance, and larger rendering work. Good starting contributions include reporting a reproducible window-manager interaction, improving a diagnostic message, adding a missing regression scenario, testing a real XLibre session, or improving either language of the documentation.

For a new GPU backend, synchronization model, or configuration format, open an issue describing the proposed boundary and tradeoffs before starting a large implementation. Small fixes can go directly to a pull request. A feature should not advertise support merely because an extension's version can be queried.

## Report a problem

Include the commit (`git rev-parse HEAD`), distribution, server and version, window manager, GPU/driver, relevant configuration, and the smallest reproduction. Attach `compust --diagnose` output and relevant `RUST_LOG=compust=debug` logs. Describe the expected image and the actual image; a screenshot or short recording helps for rendering defects.

For performance reports, include resolution, monitor refresh rates, window count, workload, blur radius, compositor configuration, and whether the CPU measurement includes the X server. XRender can move work into that process. Compare release builds on the same machine and workload, and record the picom version and backend when making comparisons.

## Prepare a pull request

Keep each pull request focused on a behavior that can be explained and tested. Describe the trigger, the resulting behavior, how you verified it, and any known limits. Add a regression test for a reproducible defect when the test can distinguish it. Run formatting, Clippy, and the relevant tests before submitting; CI also builds the release binary.

The crate forbids local `unsafe` code. Keep X11 resource lifetimes explicit, preserve cleanup on partial initialization, and handle windows that disappear between requests. Do not broaden ignored protocol errors to make a test pass. Keep pure animation calculations separate from server interaction, and avoid allocating per frame when a reusable buffer works.

Update the English and pt-BR documentation together when behavior changes. If you cannot translate a passage confidently, say which counterpart needs help in the PR. No generated screenshots, `target/`, private desktop captures, or credentials belong in a commit. Selected test images can be generated with:

```sh
COMPUST_ARTIFACTS=artifacts cargo test --test x11
```

The project uses the [MIT license](LICENSE). Contributions should be compatible with it; identify any third-party code and its license. Naming picom as inspiration does not authorize copying source under incompatible terms.
