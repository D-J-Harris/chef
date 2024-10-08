# `chef`

Bytecode interpreter for the programming language `chef`, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/)

Bootstrapped in Rust (the book uses C), using only `std`

## Installation

```rust
cargo install chef
```

## Usage

```rust
cargo run --features debug_print_code ./example.chef
```

## Features

- `debug_print_code` - print out the disassembled chunk at the end of the compile step
- `debug_trace_execution` - print out each disassembled instruction during the interpret step

## TODO

- [ ] Change `Operation` and `Value` enums to not carry data through use of `union`. Should `Value` remain copy or cloned through some reference counting?
- [ ] Move from HashMap for identifiers (and parse rules) to a trie structure
- [ ] Macros and better runtime and compile errors for `Vm`
- [ ] Understand more about the performance and practical differences between String cloning (Rust) and the String heap allocation exercise from the "Strings" chapter (C). Does this implementation need a GC?
- [ ] Clean up chunk debugging once `Operation` has been amending to carry no data
- [ ] Change Option in fixed sized arrays to MaybeUninit and measure performance improvements (unsafe)
- [ ] Add String interning

## Challenges

The book notes a number of stretch challenges, which I have compiled below

- [ ] Devise an encoding that compresses the line information for a series of instructions on the same line
- [ ] Add `OP_CONSTANT_LONG` operation

> This leads us to optimising the size of constant `Value` so that no space is wasted on smaller constants. We could split up our arrays here to hold types of similar size, at the cost of managing more state and potentially needing to dynamically grow constant arrays more frequently

- [ ] Look into [flexible array members](https://en.wikipedia.org/wiki/Flexible_array_member)
- [ ] Add support for discerning between string literals that point back to source code and those that own their char array, to save memory on the heap for these cases
- [ ] Support resolve variable scanning through a more efficient DS
- [ ] Add support for switch statement and continue clauses in for loops

## Decisions

- Just like the source material, the character set is restricted to UTF-8 which enables us to scan the source code one byte at a time. The encoding of the source code is checked to be UTF-8 at runtime
