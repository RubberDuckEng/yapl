use super::*;

pub fn println(args: &Map<String, Value>) -> Result<Value, Error> {
    println!("{}", serialize(&args["msg"])?);
    Ok(Value::Null)
}
