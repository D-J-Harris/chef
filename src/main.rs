use std::env;
use std::io;
use std::io::Write;
use std::process::exit;

use scanner::token::TokenKind;
use scanner::Scanner;
use vm::Vm;

mod chunk;
mod compiler;
mod scanner;
mod value;
mod vm;

fn main() {
    let mut vm = Vm::new();
    let args = env::args().collect::<Vec<String>>();

    match args.len() {
        1 => repl(vm),
        2 => run_file(vm, &unsafe { args.get_unchecked(1) }),
        0 | 3.. => {
            eprintln!("Usage: chef [path]");
            exit(64)
        }
    }
}

fn repl(vm: Vm) {
    let mut buf = String::new();
    loop {
        print!("chef > ");
        io::stdout().flush().expect("Could not write to stdout");
        std::io::stdin()
            .read_line(&mut buf)
            .expect("Could not read user input");
        // TODO: interpret
        println!("{}", buf.strip_suffix('\n').unwrap());
    }
}

fn run_file(vm: Vm, path: &str) {
    let Ok(mut source) = std::fs::read_to_string(path) else {
        eprintln!("Could not read File");
        exit(74);
    };
    source.push('\0');
    let mut scanner = Scanner::new(source.as_str());
    loop {
        let token = scanner.scan_token();
        println!("{token:?}");
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    // TODO: exit codes for compile or runtime errors
}
