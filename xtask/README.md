# xtask

This directory contains development tasks for the termlib-server project using the [xtask pattern](https://github.com/matklad/cargo-xtask).

## Usage

Run tasks from the project root directory:

```bash
cargo xtask <command>
```

## Available Commands

### `format`
Formats all code in the workspace using `cargo fmt`.

```bash
cargo xtask format
```

### `lint`
Runs clippy lints on all targets with all features enabled, treating warnings as errors.

```bash
cargo xtask lint
```

### `test`
Runs all tests in the workspace.

```bash
cargo xtask test
```

### `build`
Builds the project in release mode.

```bash
cargo xtask build
```

### `dev`
Starts the development server (equivalent to `cargo run`).

```bash
cargo xtask dev
```

### `clean`
Cleans all build artifacts.

```bash
cargo xtask clean
```

### `ci`
Runs the complete CI pipeline: format, lint, test, and build.

```bash
cargo xtask ci
```

## Adding New Tasks

To add new development tasks:

1. Add a new variant to the `Commands` enum in `src/main.rs`
2. Add a corresponding match arm in the `main()` function
3. Implement the task function following the existing patterns
4. Update this README with documentation for the new command

## Why xtask?

The xtask pattern provides several benefits:

- **Consistency**: All developers use the same commands regardless of their local setup
- **Discoverability**: All available tasks are listed in one place with `cargo xtask --help`
- **Flexibility**: Complex build logic can be written in Rust rather than shell scripts
- **Cross-platform**: Works consistently across different operating systems
