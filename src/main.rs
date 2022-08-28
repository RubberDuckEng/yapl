use rustyline;

fn main() -> rustyline::Result<()> {
    let mut rl = rustyline::Editor::<()>::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => println!("Line: {:?}", line),
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
