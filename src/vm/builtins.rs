use super::*;

pub fn println(args: &Map<String, Arc<Value>>) -> Result<Arc<Value>, Error> {
    println!("{}", Value::as_string(&args["msg"])?);
    Ok(Value::null())
}
