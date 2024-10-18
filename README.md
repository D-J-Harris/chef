# `chef`

Custom programming language `chef`, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/) and porting the C bytecode interpreter into Rust

TODO: Please find a write-up [here]()

## Installation

TODO: Package and release `chef`

```rust
cargo install chef
```

## Usage

Run either with

- zero arguments (REPL)
- one argument, which runs a single file

```rust
chef
chef <.chef file>
```

## Features Flags

- `--debug_trace` - print out each disassembled chunk at compile time, and each operation during runtime

## TODO

- [ ] Fix up debug print formatting
- [ ] Add string interning
- [ ] Remove explicit checks for operations the VM should trust the compiler on
- [ ] Can move upvalues (`is_local` and `index` information) to `Function` object, instead of emitting operations

## Performance TODO

- [ ] Change `Operation` enum to use one byte only (cache locality)
- [ ] Move from HashMap for identifiers (and parse rules) to a trie structure
- [ ] Can fixed sized arrays be allocated more efficicently than using `Option<T>`
- [ ] Move array sizes from `U8_COUNT_USIZE` to `u8::MAX`, encode counts as `u8`

## Features TODO

- [ ] `switch` statement
- [ ] `continue` statement in loops
- [ ] Support for lists
- [ ] Support for native method to read from a file

## Challenges

- [ ] Devise an encoding that compresses the line information for a series of instructions on the same line
- [ ] Look into [flexible array members](https://en.wikipedia.org/wiki/Flexible_array_member)
- [ ] Discerning between string literals that point back to source code and those require heap allocation
- [ ] Support resolve variable scanning through a more efficient DS
- [ ] Instruction pointer is accessed a lot, amend to encourage compiler to put it in registers
- [ ] Add arity checking and runtime error handling for native functions

## Garbage Collection

This language implementation does not implement a garbage collector, but implicitly does so through reference counting in the underlying Rust implementation.

## Test

```sh
cargo test
```

The tests differ to the original test suite in the following ways:

- `scanning/*.lox` is removed
- `expressions/parse.lox` is removed
- `operator/equals_method.lox` second assertion changed to expect true

## Copyright Notice

Codebases and references all MIT licensed, including [this repository](./LICENSE)

- [Test and benchmark files](./tests/suite/) adapted from the [book](https://github.com/munificent/craftinginterpreters)
- [Test suite runner code](./tests/run.rs) inspired from [loxido](https://github.com/ceronman/loxido/tree/unsafe)
