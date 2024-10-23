#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use compiler::Compiler;
use error::ChefError;
use error::InterpretResult;
use gc_arena::arena::CollectionPhase;
use gc_arena::Arena;
use gc_arena::Gc;
use gc_arena::Rootable;
use objects::ClosureObject;
use strings::StringInterner;
use value::Value;
use vm::State;

mod chunk;
mod common;
mod compiler;
mod error;
mod native_functions;
mod objects;
mod rules;
mod scanner;
mod strings;
mod value;
mod vm;
pub struct Chef {
    state: Arena<Rootable![State<'_>]>,
}

impl<'source> Chef {
    fn new() -> Self {
        let arena = Arena::<Rootable![State<'_>]>::new(|mc| {
            let mut state = State::new(mc, StringInterner::new(mc));
            state.declare_native_functions();
            state
        });

        Self { state: arena }
    }

    fn interpret(&mut self, source: &'source str) -> InterpretResult<()> {
        const COLLECTOR_STEPS: u32 = 4096;

        self.state.mutate_root(|mc, state| {
            let compiler = Compiler::new(mc, source, state);
            let function = compiler.compile().ok_or(ChefError::Compile)?;
            state.push(Value::Function(function))?;
            let closure = Gc::new(mc, ClosureObject::new(function.upvalue_count, function));
            let call_frame = state.call(closure, 0)?;
            state.push_frame(call_frame)
        })?;

        #[cfg(feature = "debug_trace")]
        println!("====== Executing      ======");

        const COLLECTOR_GRANULARITY: f64 = 1024.0;
        loop {
            match self.state.mutate_root(|_, state| {
                let result = state.run(COLLECTOR_STEPS);
                if let Err(err) = &result {
                    eprintln!("{err}");
                    state.stack_error();
                }
                result
            }) {
                Ok(false) => {
                    if self.state.metrics().allocation_debt() > COLLECTOR_GRANULARITY {
                        if self.state.collection_phase() == CollectionPhase::Sweeping {
                            self.state.collect_debt();
                        } else {
                            // Immediately transition to `CollectionPhase::Sweeping`.
                            self.state.mark_all().unwrap().start_sweeping();
                        }
                        continue;
                    }
                }
                result => break result.map(|_| ()),
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

    // unix sysexits.h exit codes
    match chef.interpret(&source) {
        Ok(_) => exit(0),
        Err(ChefError::Compile) => exit(65),
        Err(_) => exit(70),
    }
}
