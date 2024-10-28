use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use compiler::Compiler;
use error::ChefError;
use error::InterpretResult;
use native_functions::declare_native_functions;
use value::Value;
use vm::State;

mod chunk;
mod common;
mod compiler;
mod error;
mod function;
mod native_functions;
mod rules;
mod scanner;
mod value;
mod vm;

fn interpret<'src>(source: &'src str) -> InterpretResult<()> {
    let mut state = State::new();
    declare_native_functions(&mut state.globals);
    let compiler = Compiler::new(source);
    let function = compiler.compile().ok_or(ChefError::Compile)?;
    state.push(Value::Function(function))?;
    state.call(0)?;
    let result = state.run();
    if let Err(err) = &result {
        eprintln!("{err}");
        state.stack_error();
    }
    result
}

fn main() {
    let args = env::args().collect::<Vec<String>>();

    match args.len() {
        1 => repl(),
        2 => run_file(&args[1]),
        _ => {
            eprintln!("Usage: chef [path]");
            exit(64)
        }
    }
}

fn repl() {
    let mut buf = String::new();
    loop {
        buf.clear();
        print!("chef > ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buf).unwrap();
        buf.push('\0');
        let _ = interpret(&buf);
    }
}

fn run_file(path: &str) {
    let Ok(mut source) = std::fs::read_to_string(path) else {
        eprintln!("Could not read File");
        exit(74);
    };
    source.push('\0');

    // unix sysexits.h exit codes
    match interpret(&source) {
        Ok(_) => exit(0),
        Err(ChefError::Compile) => exit(65),
        Err(_) => exit(70),
    }
}
