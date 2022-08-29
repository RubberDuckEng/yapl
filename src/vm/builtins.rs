use super::*;

pub fn println(args: &Map<String, Value>) -> Result<Value, Error> {
    println!("{}", as_string(&args["msg"])?);
    Ok(Value::Null)
}
