use super::*;

pub fn println(args: &Arc<Value>) -> Result<Arc<Value>, Error> {
    println!("{}", Value::as_string(args)?);
    Ok(Value::null())
}

pub fn deserialize(args: &Arc<Value>) -> Result<Arc<Value>, Error> {
    let string = Value::as_string(&args)?;
    Ok(parse(&string)?)
}

pub fn serialize(args: &Arc<Value>) -> Result<Arc<Value>, Error> {
    Ok(Arc::new(Value::String(super::serialize(args)?)))
}

fn get_formals(args: &Arc<Value>) -> Result<Formals, Error> {
    match args.as_ref() {
        Value::String(name) => Ok(Formals::Singleton(name.clone())),
        Value::Array(names) => {
            let strings: Result<Vec<&str>, Error> =
                names.iter().map(|name| Value::as_string(name)).collect();
            Ok(Formals::Positional(
                strings?.iter().map(|name| name.to_string()).collect(),
            ))
        }
        Value::Object(names) => Ok(Formals::Named(
            names.keys().map(|name| name.clone()).collect(),
        )),
        _ => Err(Error::TypeError),
    }
}

pub fn lambda(
    env: &Arc<Environment>,
    object: &Object,
    args: &Arc<Value>,
) -> Result<Arc<Value>, Error> {
    Ok(Arc::new(Value::Function(Arc::new(Function {
        body: FunctionBody::Lambda(Lambda {
            env: env.clone(),
            formals: get_formals(args)?,
            body: get_key(object, "body")?.clone(),
        }),
    }))))
}

pub fn lookup(
    env: &Arc<Environment>,
    _object: &Object,
    args: &Arc<Value>,
) -> Result<Arc<Value>, Error> {
    // TODO: Support pathing operators.
    Ok(env.lookup(Value::as_string(args)?)?.clone())
}

pub fn quote(
    _env: &Arc<Environment>,
    _object: &Object,
    args: &Arc<Value>,
) -> Result<Arc<Value>, Error> {
    Ok(args.clone())
}
