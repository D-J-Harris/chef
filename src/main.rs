use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use vm::Vm;

mod chunk;
mod common;
mod compiler;
mod error;
mod native_functions;
mod objects;
mod scanner;
mod value;
mod vm;

fn main() {
    let vm = Vm::new();
    let args = env::args().collect::<Vec<String>>();

    match args.len() {
        1 => repl(vm),
        2 => run_file(vm, &args[1]),
        _ => {
            eprintln!("Usage: chef [path]");
            exit(64)
        }
    }
}

fn repl(mut vm: Vm) {
    let mut buf = String::new();
    loop {
        buf.clear();
        print!("chef > ");
        io::stdout().flush().expect("Could not flush stdout.");
        io::stdin()
            .read_line(&mut buf)
            .expect("Could not read user input.");
        buf.push('\0');
        vm.interpret(&buf);
    }
}

fn run_file(mut vm: Vm, path: &str) {
    let Ok(mut source) = std::fs::read_to_string(path) else {
        eprintln!("Could not read File");
        exit(74);
    };
    source.push('\0');
    match vm.interpret(&source) {
        vm::InterpretResult::Ok => (),
        vm::InterpretResult::CompileError => exit(65),
        vm::InterpretResult::RuntimeError => exit(70),
    }
}
