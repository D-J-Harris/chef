# `chef`

This branch hosts the core Lox implementation, created following Part III of the book [Crafting Interpreters](https://craftinginterpreters.com/) and porting the C bytecode interpreter into Rust

## Usage

Run either with zero arguments as a REPL, or with one file

```rust
chef
chef <.lox file>
```

## Features Flags

- `--debug_code` - print out each disassembled chunk at the end of compile time
- `--debug_trace` - print out each disassembled operation during runtime

## TODO

- [x] Add string interning
- [x] Increase jump opcode distance to original u16 maximum
- [x] Change `Operation` enum to use one byte only (cache locality improvement)
- [ ] Can move upvalues (`is_local` and `index` information) to `Function` object, instead of emitting operations
- [ ] Move from HashMap for identifiers to a trie structure
- [ ] Move array sizes from `U8_COUNT_USIZE` to `u8::MAX`, encode counts as `u8` rather than `usize`

## Features TODO

- [ ] `switch` statement
- [ ] `continue` statement in loops
- [ ] Support for lists
- [ ] Support for native method to read from a file

## Test

```sh
cargo test
```

The tests differ to the original test suite in the following ways:

- `scanning/*.lox` is removed
- `expressions/parse.lox` is removed

## Benchmarks

Running `./run_benchmark.sh` shell script will run each benchmark test 5 times and output results to a local `.csv` file

- List variable `BINARIES` defined in the script to control which binaries are run
- List variable `BENCHMARK_FILES` defined in the script to control which benchmarks are run

The local `./benchmark_plot.py` file can be used to visualise these CSV files on a group bar chart. This runs as a `uv` package manager script:

```shell
uv run benchmark_plot.py --output plot.png <space-separated-csv-file-paths>
```

## Copyright Notice

Codebases and references all MIT licensed, including [this repository](./LICENSE)

- [Test and benchmark files](./tests/suite/) adapted from the [book](https://github.com/munificent/craftinginterpreters)
- [Test suite runner code](./tests/run.rs) inspired from [loxido](https://github.com/ceronman/loxido/tree/unsafe)

## Patches

`gc-arena` crate is patched in the following ways (pointer equality and hashing)

```rust
impl<'gc, T: PartialEq + ?Sized + 'gc> PartialEq for Gc<'gc, T> {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(*self, *other)
    }

    fn ne(&self, other: &Self) -> bool {
        !Gc::ptr_eq(*self, *other)
    }
}

impl<'gc, T: Hash + ?Sized + 'gc> Hash for Gc<'gc, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(Gc::as_ptr(*self), state);
    }
}
```

These are such that the pointer type `Gc<'gc, T>` can be used as a hash key in hashmap implementations
