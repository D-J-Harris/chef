# `chef`

This branch hosts the core Lox implementation, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/) and porting the C bytecode interpreter into Rust

## Usage

Run either with zero arguments as a REPL, or with one file

```rust
chef
chef <.chef file>
```

## Features Flags

- `--debug_code` - print out each disassembled chunk at the end of compile time
- `--debug_trace` - print out each disassembled operation during runtime

## Test

```sh
cargo test
```

## Copyright Notice

Codebases and references all MIT licensed, including [this repository](./LICENSE)

- [Test and benchmark files](./tests/suite/) adapted from the [book](https://github.com/munificent/craftinginterpreters)
- [Test suite runner code](./tests/run.rs) inspired from [loxido](https://github.com/ceronman/loxido/tree/unsafe)

```

```
