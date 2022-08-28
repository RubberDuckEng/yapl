use rustyline;

mod vm;

fn main() -> rustyline::Result<()> {
    let mut rl = rustyline::Editor::<()>::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => match vm::parse(&line).and_then(|value| vm::serialize(&value)) {
                Ok(json) => println!("{}", json),
                Err(err) => println!("Error: {:?}", err),
            },
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => println!("Error: {:?}", err),
        }
    }
    Ok(())
}
