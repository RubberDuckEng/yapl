extern crate jsonpl;

use jsonpl::vm;
use rustyline;
use std::sync::Arc;

fn read_eval_print(env: &Arc<vm::Environment>, input: &str) -> Result<String, vm::Error> {
    let value = vm::parse(input)?;
    let value = vm::eval(env, &value)?;
    vm::serialize(&value)
}

fn main() -> rustyline::Result<()> {
    let mut rl = rustyline::Editor::<()>::new()?;
    let env = vm::Environment::builtin(".".to_string());
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => match read_eval_print(&env, &line) {
                Ok(json) => println!("{}", json),
                Err(err) => println!("Error: {:?}", err),
            },
            Err(rustyline::error::ReadlineError::Interrupted) => {
                eprintln!("CTRL-C");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                eprintln!("CTRL-D");
                break;
            }
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }
    Ok(())
}
