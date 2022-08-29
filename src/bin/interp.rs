extern crate jsonpl;

use jsonpl::vm;
use std::env;
use std::fs;
use std::process::exit;

fn main() -> Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        exit(1);
    }
    let path = &args[1];
    let input = fs::read_to_string(path)?;
    let env = vm::Environment::builtin();
    match vm::parse(&input).and_then(|value| vm::eval(&env, &value)) {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!("Error: {:?}", err);
            exit(1);
        }
    }
}
