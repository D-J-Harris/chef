use std::path::PathBuf;
use std::{env, process::Command};

use regex::Regex;
use test_generator::test_resources;

fn command() -> Command {
    // Create full path to binary
    let path = env::current_exe().expect("Could not get path to current executable.");
    let path = path.parent().expect("Path parent not found.");
    let mut path = path.parent().expect("Path parent not found.").to_owned();
    path.push(env!("CARGO_PKG_NAME"));
    path.set_extension(env::consts::EXE_EXTENSION);
    Command::new(path.into_os_string())
}

struct RuntimeError {
    line_prefix: String,
    message: String,
}

struct Expected {
    out: Vec<String>,
    compile_err: Vec<String>,
    runtime_err: Option<RuntimeError>,
}

fn parse_comments(path: &PathBuf) -> Expected {
    let output_re = Regex::new(r"// expect: ?(.*)").expect("Invalid regex.");
    let error_re = Regex::new(r"// (Error.*)").expect("Invalid regex.");
    let error_line_re = Regex::new(r"// \[(?:c )?line (\d+)\] (Error.*)").expect("Invalid regex.");
    let runtime_error_re = Regex::new(r"// expect runtime error: (.+)").expect("Invalid regex.");

    let mut expected = Expected {
        out: vec![],
        compile_err: vec![],
        runtime_err: None,
    };

    println!("{}", path.display());
    let content = std::fs::read_to_string(path).expect("Could not read path to string.");
    for (i, line) in content.lines().enumerate() {
        if let Some(m) = output_re.captures(line) {
            let s = m[1].to_owned();
            expected.out.push(s);
        }
        if let Some(m) = error_line_re.captures(line) {
            let line = &m[1];
            let msg = &m[2];
            let s = format!("[line {}] {}", line, msg);
            expected.compile_err.push(s);
        }
        if let Some(m) = error_re.captures(line) {
            let msg = &m[1];
            let s = format!("[line {}] {}", i + 1, msg);
            expected.compile_err.push(s);
        }
        if let Some(m) = runtime_error_re.captures(line) {
            let message = m[1].to_owned();
            let line_prefix = format!("[line {}]", i + 1);
            expected.runtime_err = Some(RuntimeError {
                line_prefix,
                message,
            });
        }
    }
    expected
}

#[test_resources("tests/suite/assignment/*.lox")]
fn run_file_test(filename: &str) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(filename);
    let expected = parse_comments(&path);
    let output = command()
        .arg(path)
        .output()
        .expect("Command execution error.");

    let out: Vec<String> = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8")
        .lines()
        .map(|x| x.to_owned())
        .collect();
    let err: Vec<String> = String::from_utf8(output.stderr)
        .expect("Invalid UTF-8")
        .lines()
        .map(|x| x.to_owned())
        .collect();

    match (
        expected.runtime_err.is_none(),
        expected.compile_err.is_empty(),
    ) {
        (true, true) => assert!(
            output.status.success(),
            "Program exited with failure, expected success"
        ),
        (false, true) => assert_eq!(
            output
                .status
                .code()
                .expect("Process terminated by a signal."),
            70,
            "Runtime errors should have error code 70"
        ),
        (true, false) => assert_eq!(
            output
                .status
                .code()
                .expect("Process terminated by a signal."),
            65,
            "Compile errors should have error code 65"
        ),
        (false, false) => panic!("Simultaneous error and compile error"),
    }

    if let Some(e) = expected.runtime_err {
        assert_eq!(e.message, err[0], "Runtime error should match");
        assert_eq!(
            err[1][0..e.line_prefix.len()],
            e.line_prefix,
            "Runtime error line should match"
        );
    } else {
        if !err.is_empty() {
            assert_eq!(
                output
                    .status
                    .code()
                    .expect("Process terminated by a signal."),
                65,
                "Compile errors should have error code 65"
            );
        }
        assert_eq!(expected.compile_err, err, "Compile error should match");
    }

    assert_eq!(expected.out, out, "Output should match");
}
