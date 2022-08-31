use super::*;
use std::fs;
use std::path::Path;

pub fn println(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    println!("{}", Value::as_string(args)?);
    Ok(Value::null())
}

pub fn print(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    print!("{}", Value::as_string(args)?);
    Ok(Value::null())
}

pub fn deserialize(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    let string = Value::as_string(&args)?;
    Ok(parse(&string)?)
}

pub fn serialize(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    Ok(Value::String(Arc::new(super::serialize(args)?)))
}

fn get_formals(args: &Value) -> Result<Formals, Error> {
    match args {
        Value::String(name) => Ok(Formals::Singleton(name.clone())),
        Value::Array(names) => {
            let strings: Result<Vec<&Arc<String>>, Error> =
                names.iter().map(|name| Value::as_string(name)).collect();
            Ok(Formals::Positional(
                strings?.iter().map(|name| Arc::clone(name)).collect(),
            ))
        }
        Value::Object(names) => Ok(Formals::Named(
            names.keys().map(|name| Arc::new(name.clone())).collect(),
        )),

        _ => Err(Error::invalid_type(
            "Formal parameters (string, array, or object)",
            args,
        )),
    }
}

pub fn lambda(env: &Arc<Env>, object: &Object, args: &Value) -> Result<Value, Error> {
    Ok(Value::Function(Arc::new(Function {
        body: FunctionBody::Lambda(Lambda {
            env: env.clone(),
            formals: get_formals(args)?,
            body: get_key(object, "+in")?.clone(),
        }),
    })))
}

pub fn lookup(env: &Arc<Env>, _object: &Object, args: &Value) -> Result<Value, Error> {
    // TODO: Support pathing operators.
    Ok(env.lookup(Value::as_string(args)?)?.clone())
}

pub fn quote(_env: &Arc<Env>, _object: &Object, args: &Value) -> Result<Value, Error> {
    Ok(args.clone())
}

// TODO: In this version of let, the values being bound to variables cannot see
// themselves or other variables being bound. Eventually, we'll want letrec,
// which will allow variables to see other variables, but involves a mutation
// somewhere.
pub fn nonrecursive_let(env: &Arc<Env>, object: &Object, args: &Value) -> Result<Value, Error> {
    let bindings = Value::as_object(args)?;
    let variables = bindings
        .iter()
        .map(|(name, value)| {
            let value = eval(env, value)?;
            Ok((name.clone(), value))
        })
        .collect::<Result<ObjectMap, Error>>()?;
    let child_env = Env::new(variables, Some(env.clone()));
    eval(&child_env, get_key(object, "+in")?)
}

pub fn import(env: &Arc<Env>, object: &Object, args: &Value) -> Result<Value, Error> {
    let mut variables = ObjectMap::new();
    let modules = Value::as_object(args)?;
    let file_path = Path::new(Value::as_str(env.lookup(FILE_SYMBOL)?)?);
    let file_dir = file_path.parent().unwrap();
    for (name, value) in modules.iter() {
        let path_name = format!("{}.yapl", name);
        let path = file_dir.join(path_name);
        let program = fs::read_to_string(&path).map_err(|_| Error::IO)?;
        let parsed_program = parse(&program)?;
        let root_env = Env::builtin(path.display().to_string());
        let exports = eval(&root_env, &parsed_program)?;
        match value {
            Value::String(name) => {
                variables.insert(name.to_string(), exports);
            }
            Value::Null => {
                for (name, value) in Value::as_object(&exports)?.iter() {
                    variables.insert(name.to_string(), value.clone());
                }
            }
            _ => {
                return Err(Error::invalid_type(
                    "import mapping (string or null)",
                    &exports,
                ))
            }
        };
    }
    let child_env = Env::new(variables, Some(env.clone()));
    eval(&child_env, get_key(object, "+in")?)
}

pub fn export(env: &Arc<Env>, _object: &Object, args: &Value) -> Result<Value, Error> {
    let object = Value::as_object(args)?;
    Ok(Value::Object(eval_object(env, object)?))
}

pub fn map(env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    let args = Value::as_array(args)?;
    let func = Value::as_function(get_index(args, 0)?)?;
    let array = Value::as_array(get_index(args, 1)?)?;

    let results = array
        .iter()
        .map(|value| func.call(env, value))
        .collect::<Result<Vec<Value>, Error>>()?;
    Ok(Value::Array(Arc::new(results)))
}

pub fn if_func(env: &Arc<Env>, object: &Object, args: &Value) -> Result<Value, Error> {
    let condition = Value::as_bool(&eval(env, args)?)?;
    if condition {
        eval(env, get_key(object, "+then")?)
    } else {
        eval(env, get_key(object, "+else")?)
    }
}

pub fn eq(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    let args = Value::as_array(args)?;
    let lhs = get_index(args, 0)?;
    let rhs = get_index(args, 1)?;
    Ok(Value::Bool(lhs == rhs))
}

pub fn plus(_env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
    let args = Value::as_array(args)?;
    let lhs = Value::as_f64(get_index(args, 0)?)?;
    let rhs = Value::as_f64(get_index(args, 1)?)?;
    Ok(Value::Number(Number::from_f64(lhs + rhs).unwrap()))
}
