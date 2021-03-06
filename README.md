# lolbench

CI Status: [![CircleCI](https://circleci.com/gh/anp/lolbench/tree/master.svg?style=shield)](https://circleci.com/gh/anp/workflows/lolbench)

This project is an effort to reproducibly benchmark "in the wild" Rust code against newer compiler versions to detect performance regressions. The data is currently published to [https://lolbench.rs](https://lolbench.rs).

## Adding Benchmarks

Want to contribute and are looking for the [list of benchmarks we'd like help adding](https://github.com/anp/lolbench/issues/1)?

## Getting Started

### Dependencies

- rustup
- clang (Linux only)

### Building & Running

```
$ git submodule update --init
$ cargo test-core
```

### Useful cargo subcommand aliases

- `cargo build-website` runs the website generator using the provided data directory. Pass the `--help` flag to see what's required.
- `cargo fmt-core` formats only those crates which should be rustfmt'd -- notably our fork of criterion isn't rustfmt-friendly right now. Useful for `cargo watch -x fmt-core`.
- `cargo test-core` runs the tests for every non-benchmark crate except for criterion. At writing, that's `lolbench`, `lolbench_support`, `lolbench_extractor`, and `marky_mark`.
- `cargo new-bench-crate` runs a lolbench command to create a new benchmark crate in the benches directory.
- `cargo build-all [--release]` builds a binary for every benchmark function. _caution: this will generate dozens of gigabytes of data in your target directory_.
- `cargo test-all [--release]` runs the test for every benchmark function, which consists of warming it up and running through a couple of iterations. _caution: this will generate dozens of gigabytes of data in your target directory_.

## License

lolbench is distributed under the terms of both the MIT license and the Apache License (Version 2.0). Some included benchmarks have their own separate licenses, see those directories and their Cargo.toml metadata for details.
