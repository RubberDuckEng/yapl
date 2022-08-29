use super::*;

pub fn println(args: &Object) -> Result<Arc<Value>, Error> {
    println!("{}", Value::as_string(&args["msg"])?);
    Ok(Value::null())
}

pub fn deserialize(args: &Object) -> Result<Arc<Value>, Error> {
    let string = Value::as_string(&args["string"])?;
    Ok(parse(&string)?)
}

pub fn serialize(args: &Object) -> Result<Arc<Value>, Error> {
    let value = &args["value"];
    Ok(Arc::new(Value::String(super::serialize(value)?)))
}

pub fn lambda(env: &Arc<Environment>, object: &Object) -> Result<Arc<Value>, Error> {
    let formals = Value::as_array(get_key(object, "formals")?)?
        .into_iter()
        .map(|value| Ok(Value::as_string(value)?.to_string()))
        .collect::<Result<Vec<String>, Error>>()?;
    let body = get_key(object, "body")?;
    Ok(Arc::new(Value::Function(Arc::new(Function {
        body: FunctionBody::Lambda(Lambda {
            env: env.clone(),
            formals,
            body: body.clone(),
        }),
    }))))
}

pub fn lookup(env: &Arc<Environment>, object: &Object) -> Result<Arc<Value>, Error> {
    let symbol = Value::as_string(get_key(object, "symbol")?)?;
    Ok(env.lookup(symbol)?.clone())
}

pub fn quote(_env: &Arc<Environment>, object: &Object) -> Result<Arc<Value>, Error> {
    get_key(object, "object").map(Arc::clone)
}
