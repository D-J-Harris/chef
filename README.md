# `chef`

Bytecode interpreter for the programming language `chef`, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/)

Bootstrapped in Rust (the book uses C), using only `std`

## Installation

```rust
cargo install chef
```

## Usage

```rust
cargo run --features debug-print-code ./example.chef
```

## Features

- `debug-print-code` - print out the disassembled chunk at the end of the compile step
- `debug-trace-execution` - print out each disassembled instruction during the interpret step

## TODO

- [ ] Change `Operation` and `Value` enums to not carry data through use of `union`
- [ ] Move from HashMap for identifiers (and parse rules) to a trie structure
- [ ] Macros and better runtime and compile errors for `Vm`

## Challenges

The book notes a number of stretch challenges, which I have compiled below

- [ ] Devise an encoding that compresses the line information for a series of instructions on the same line
- [ ] Add `OP_CONSTANT_LONG` operation

> This leads us to optimising the size of constant `Value` so that no space is wasted on smaller constants. We could split up our arrays here to hold types of similar size, at the cost of managing more state and potentially needing to dynamically grow constant arrays more frequently

## Decisions

- Just like the source material, the character set is restricted to UTF-8 which enables us to scan the source code one byte at a time. The encoding of the source code is checked to be UTF-8 at runtime
