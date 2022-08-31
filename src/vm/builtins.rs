use super::*;
use std::fs;
use std::path::Path;

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
            body: get_key(object, "+in")?.clone(),
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

// TODO: In this version of let, the values being bound to variables cannot see
// themselves or other variables being bound. Eventually, we'll want letrec,
// which will allow variables to see other variables, but involves a mutation
// somewhere.
pub fn nonrecursive_let(
    env: &Arc<Environment>,
    object: &Object,
    args: &Arc<Value>,
) -> Result<Arc<Value>, Error> {
    let bindings = Value::as_object(args)?;
    let variables: Object = bindings
        .iter()
        .map(|(name, value)| {
            let value = eval(env, value)?;
            Ok((name.clone(), value))
        })
        .collect::<Result<Object, Error>>()?;
    let child_env = Arc::new(Environment {
        variables: variables,
        parent: Some(env.clone()),
    });
    eval(&child_env, get_key(object, "+in")?)
}

pub fn import(
    env: &Arc<Environment>,
    object: &Object,
    args: &Arc<Value>,
) -> Result<Arc<Value>, Error> {
    let mut variables = Object::new();
    let modules = Value::as_object(args)?;
    let file_path = Path::new(Value::as_string(env.lookup(FILE_SYMBOL)?)?);
    let file_dir = file_path.parent().unwrap();
    for (name, _value) in modules.iter() {
        let path_name = format!("{}.yapl", name);
        let path = file_dir.join(path_name);
        let program = fs::read_to_string(&path).map_err(|_| Error::IOError)?;
        let parsed_program = parse(&program)?;
        let root_env = Environment::builtin(path.display().to_string());
        let exports = eval(&root_env, &parsed_program)?;
        match exports.as_ref() {
            Value::String(name) => {
                variables.insert(name.clone(), exports);
            }
            Value::Null => {
                for (name, value) in Value::as_object(&exports)?.iter() {
                    variables.insert(name.clone(), value.clone());
                }
            }
            _ => return Err(Error::TypeError),
        };
    }
    let child_env = Arc::new(Environment {
        variables: variables,
        parent: Some(env.clone()),
    });
    eval(&child_env, get_key(object, "+in")?)
}
