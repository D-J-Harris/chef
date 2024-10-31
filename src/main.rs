use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use compiler::Compiler;
use error::ChefError;
use error::InterpretResult;
use vm::CallFrame;
use vm::State;

mod code;
mod common;
mod compiler;
mod error;
mod native_functions;
mod rules;
mod scanner;
mod value;
mod vm;

fn interpret(source: &str) -> InterpretResult<()> {
    let compiler = Compiler::new(source);
    let code = compiler.compile().ok_or(ChefError::Compile)?;
    let mut state = State::new(code);
    state.push_frame(CallFrame::default())?;
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
    if !path.ends_with(".chef") && !path.ends_with(".recipe") {
        eprintln!("Source code file extension should be `.chef` or `.recipe`.");
        exit(74);
    }
    let Ok(mut source) = std::fs::read_to_string(path) else {
        eprintln!("Could not read file.");
        exit(74);
    };
    source.push('\0');

    // unix sysexits.h exit codes
    match interpret(&source) {
        Err(ChefError::Compile) => exit(65),
        Ok(_) => exit(0),
        Err(_) => exit(70),
    }
}
