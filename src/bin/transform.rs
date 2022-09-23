extern crate jsonpl;

use anyhow::Result;
use jsonpl::vm;
use std::env;
use std::fs;
use std::io;
use std::io::Read;
use std::process::exit;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        exit(1);
    }
    let path = &args[1];
    let input = fs::read_to_string(path)?;
    let env = vm::Env::builtin(path.to_string());
    match vm::parse(&input).and_then(|value| vm::eval(&env, &value)) {
        Ok(transform) => {
            let mut input_data = String::new();
            io::stdin().read_to_string(&mut input_data)?;
            let input = vm::parse(&input_data).unwrap();
            let function = vm::Value::as_function(&transform).unwrap();
            let output_data = function.call(&env, &input).unwrap();
            println!("{}", vm::serialize(&output_data).unwrap());
            Ok(())
        }
        Err(err) => {
            eprintln!("Error: {:?}", err);
            exit(1);
        }
    }
}
