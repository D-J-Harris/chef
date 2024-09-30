# `chef`

Bytecode interpreter for the programming language `chef`, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/)

Bootstrapped in Rust (the book uses C), using only `std`

## Installation

```rust
cargo install
```

## Usage

## Features

-

## TODO

- [ ] Change `Operation` enum to not carry data. How does this impact performance, with improved cache locality of the enum
- [ ] Move from HashMap for identifiers to a trie structure
- [ ] How can performance be improved by moving into unsafe and dealing with string pointers directly?

## Challenges

The book notes a number of stretch challenges, which I have compiled below

- [ ] Devise an encoding that compresses the line information for a series of instructions on the same line
- [ ] Add `OP_CONSTANT_LONG` operation

> This leads us to optimising the size of constant `Value` so that no space is wasted on smaller constants. We could split up our arrays here to hold types of similar size, at the cost of managing more state and potentially needing to dynamically grow constant arrays more frequently

## Decisions

- Just like the source material, the character set is restricted to UTF-8 which enables us to scan the source code one byte at a time. The encoding of the source code is checked at runtime

## Debugging

Build with feature `vm-trace` to optionally print instruction disassembly to stdout
