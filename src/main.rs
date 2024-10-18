use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use compiler::Compiler;
use error::InterpretResult;
use error::RuntimeError;
use gc_arena::Arena;
use gc_arena::Gc;
use gc_arena::Rootable;
use objects::ClosureObject;
use value::Value;
use vm::State;

mod chunk;
mod common;
mod compiler;
mod error;
mod native_functions;
mod objects;
mod scanner;
mod value;
mod vm;

pub struct Chef {
    state: Arena<Rootable![State<'_>]>,
}

impl Chef {
    fn new() -> Self {
        let arena = Arena::<Rootable![State<'_>]>::new(|mc| State::new(mc));
        Self { state: arena }
    }

    fn interpret<'source>(&mut self, source: &'source str) -> InterpretResult<()> {
        const COLLECTOR_STEPS: u8 = 255;

        loop {
            match self.state.mutate_root(|mc, state| {
                let compiler = Compiler::new(mc, source);
                let function = compiler.compile().ok_or(RuntimeError::Compile)?;
                state.push(Value::Function(function))?;
                let closure = Gc::new(mc, ClosureObject::new(function.upvalue_count, function));
                state.call(closure, 0)?;
                let result = state.run(COLLECTOR_STEPS);
                if let Err(err) = &result {
                    eprintln!("{err}");
                    state.stack_error();
                }
                result
            }) {
                Ok(false) => continue,
                _ => break Ok(()),
            }
        }
    }
}

fn main() {
    let chef = Chef::new();
    let args = env::args().collect::<Vec<String>>();

    match args.len() {
        1 => repl(chef),
        2 => run_file(chef, &args[1]),
        _ => {
            eprintln!("Usage: chef [path]");
            exit(64)
        }
    }
}

fn repl(mut chef: Chef) {
    let mut buf = String::new();
    loop {
        buf.clear();
        print!("chef > ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buf).unwrap();
        buf.push('\0');
        let _ = chef.interpret(&buf);
    }
}

fn run_file(mut chef: Chef, path: &str) {
    let Ok(mut source) = std::fs::read_to_string(path) else {
        eprintln!("Could not read File");
        exit(74);
    };
    source.push('\0');
    match chef.interpret(&source) {
        Ok(_) => todo!(),
        Err(RuntimeError::Compile) => exit(65),
        Err(_) => exit(70),
    }
}
