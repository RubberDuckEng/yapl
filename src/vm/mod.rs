use lazy_static::lazy_static;
use serde_json;
use serde_yaml;
use std::sync::Arc;

mod builtins;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    AmbiguousOperation(Vec<Op>),
    ArgumentCountMismatch(usize, usize),
    InvalidIndex(usize, usize),
    InvalidNumber(Number),
    InvalidOperation(String),
    InvalidType(String),
    IO,
    MissingNamedArgument(Arc<String>),
    MissingOperation,
    Parse,
    Serialization,
    UndefinedSymbol(String),
    UnknownKey(String),
}

impl Error {
    fn invalid_type(expected: &str, actual: &Value) -> Error {
        Error::InvalidType(format!("Expected {}, got {}", expected, actual.type_of()))
    }
}

pub type ObjectMap = Map<String, Value>;
pub type Object = Arc<ObjectMap>;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type Number = serde_json::Number;
pub type NativeFunction = fn(&Arc<Env>, &Value) -> Result<Value, Error>;
pub type NativeSpecialForm = fn(&Arc<Env>, &Object, &Value) -> Result<Value, Error>;

// TODO: Use a smarter handle than Arc to store null, bool, number, and string
// without needing a heap allocation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(Arc<String>),
    Array(Arc<Vec<Value>>),
    Object(Object),
    Function(Arc<Function>),
}

impl Value {
    pub fn null() -> Value {
        Value::Null
    }

    pub fn type_of(&self) -> &str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
        }
    }

    pub fn empty_object() -> Value {
        lazy_static! {
            static ref EMPTY: Value = Value::Object(Arc::new(ObjectMap::new()));
        }
        EMPTY.clone()
    }

    pub fn as_bool(value: &Value) -> Result<bool, Error> {
        match value {
            Value::Bool(value) => Ok(*value),
            _ => Err(Error::invalid_type("bool", value)),
        }
    }

    pub fn as_number(value: &Value) -> Result<Number, Error> {
        match value {
            Value::Number(value) => Ok(value.clone()),
            _ => Err(Error::invalid_type("number", value)),
        }
    }

    pub fn as_f64(value: &Value) -> Result<f64, Error> {
        let n = Self::as_number(value)?;
        n.as_f64().ok_or_else(|| Error::InvalidNumber(n))
    }

    pub fn as_string(value: &Value) -> Result<&Arc<String>, Error> {
        match value {
            Value::String(value) => Ok(value),
            _ => Err(Error::invalid_type("string", value)),
        }
    }

    pub fn as_str(value: &Value) -> Result<&str, Error> {
        match value {
            Value::String(value) => Ok(value),
            _ => Err(Error::invalid_type("string", value)),
        }
    }

    pub fn as_function(value: &Value) -> Result<&Function, Error> {
        match value {
            Value::Function(value) => Ok(value),
            _ => Err(Error::invalid_type("function", value)),
        }
    }

    pub fn as_object(value: &Value) -> Result<&Object, Error> {
        match value {
            Value::Object(value) => Ok(value),
            _ => Err(Error::invalid_type("object", value)),
        }
    }

    pub fn as_array(value: &Value) -> Result<&Vec<Value>, Error> {
        match value {
            Value::Array(values) => Ok(values),
            _ => Err(Error::invalid_type("array", value)),
        }
    }
}

enum FunctionBody {
    Native(NativeFunction),
    Lambda(Lambda),
    NativeSpecialForm(NativeSpecialForm),
}

impl std::fmt::Debug for FunctionBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("FunctionBody")
            .field("fn", &"#code")
            .finish()
    }
}

impl std::cmp::PartialEq for FunctionBody {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::cmp::Eq for FunctionBody {}

#[derive(Debug, PartialEq, Eq)]
pub struct Function {
    body: FunctionBody,
}

impl Function {
    fn eval(&self, env: &Arc<Env>, object: &Object, args: &Value) -> Result<Value, Error> {
        match &self.body {
            FunctionBody::Native(native) => {
                // TODO: This isn't quite right. We need to be able to know whether
                // the native function takes an object, an array, or a singleton value.
                let args = eval(env, args)?;
                native(env, &args)
            }
            FunctionBody::Lambda(lambda) => lambda.eval(env, args),
            FunctionBody::NativeSpecialForm(native) => native(env, object, args),
        }
    }

    pub fn call(&self, env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
        match &self.body {
            FunctionBody::Native(native) => native(env, args),
            FunctionBody::Lambda(lambda) => lambda.call(args),
            FunctionBody::NativeSpecialForm(_) => Err(Error::InvalidOperation(
                "Cannot call special form".to_string(),
            )),
        }
    }
}

fn from_serde(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => Value::Number(value),
        serde_json::Value::String(value) => Value::String(Arc::new(value)),
        serde_json::Value::Array(values) => Value::Array(Arc::new(
            values.into_iter().map(|value| from_serde(value)).collect(),
        )),
        serde_json::Value::Object(value) => Value::Object(Arc::new(Map::from_iter(
            value
                .into_iter()
                .map(|(key, value)| (key, from_serde(value))),
        ))),
    }
}

fn to_serde(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(value.clone()),
        Value::Number(value) => serde_json::Value::Number(value.clone()),
        Value::String(value) => serde_json::Value::String(value.to_string()),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(|value| to_serde(&value)).collect())
        }
        Value::Object(value) => serde_json::Value::Object(serde_json::map::Map::from_iter(
            value
                .iter()
                .map(|(key, value)| (key.clone(), to_serde(&value))),
        )),
        Value::Function(_) => serde_json::Value::String("#function".to_string()),
    }
}

pub fn parse(json: &str) -> Result<Value, Error> {
    let value: serde_json::Value = serde_yaml::from_str(json).map_err(|_| Error::Parse)?;
    Ok(from_serde(value))
}

pub fn serialize(value: &Value) -> Result<String, Error> {
    // TODO: Avoid cloning the entire value just to serialize it.
    let value: serde_json::Value = to_serde(value);
    serde_json::to_string(&value).map_err(|_| Error::Serialization)
}

pub const FILE_SYMBOL: &str = "__file__";

#[derive(Debug, PartialEq, Eq)]
pub struct Env {
    pub variables: ObjectMap,
    pub parent: Option<Arc<Env>>,
}

impl Env {
    pub fn builtin(path: String) -> Arc<Env> {
        let mut env = Env {
            variables: ObjectMap::new(),
            parent: None,
        };
        env.bind_string(FILE_SYMBOL, path);
        env.bind_native_function("deserialize", builtins::deserialize);
        env.bind_native_function("eval", eval);
        env.bind_native_function("map", builtins::map);
        env.bind_native_function("print", builtins::print);
        env.bind_native_function("println", builtins::println);
        env.bind_native_function("serialize", builtins::serialize);
        env.bind_native_function("eq", builtins::eq);
        env.bind_native_function("+", builtins::plus);
        env.bind_native_special_form("$", builtins::lookup);
        env.bind_native_special_form("export", builtins::export);
        env.bind_native_special_form("import", builtins::import);
        env.bind_native_special_form("lambda", builtins::lambda);
        env.bind_native_special_form("let", builtins::nonrecursive_let);
        env.bind_native_special_form("quote", builtins::quote);
        env.bind_native_special_form("if", builtins::if_func);
        Arc::new(env)
    }

    pub fn new(variables: ObjectMap, parent: Option<Arc<Env>>) -> Arc<Env> {
        Arc::new(Env { variables, parent })
    }

    pub fn lookup(&self, name: &str) -> Result<&Value, Error> {
        if let Some(value) = self.variables.get(name) {
            Ok(value)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            Err(Error::UndefinedSymbol(name.to_string()))
        }
    }

    pub fn bind_string(&mut self, name: &str, string: String) {
        self.variables
            .insert(name.to_string(), Value::String(Arc::new(string)));
    }

    pub fn bind_native_function(&mut self, name: &str, function: NativeFunction) {
        self.variables.insert(
            name.to_string(),
            Value::Function(Arc::new(Function {
                body: FunctionBody::Native(function),
            })),
        );
    }

    pub fn bind_native_special_form(&mut self, name: &str, special_form: NativeSpecialForm) {
        self.variables.insert(
            name.to_string(),
            Value::Function(Arc::new(Function {
                body: FunctionBody::NativeSpecialForm(special_form),
            })),
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Formals {
    Singleton(Arc<String>),
    Positional(Vec<Arc<String>>),
    Named(Vec<Arc<String>>),
}

#[derive(Debug, PartialEq, Eq)]
struct Lambda {
    env: Arc<Env>,
    formals: Formals,
    body: Value,
}

impl Lambda {
    fn eval(&self, env: &Arc<Env>, args: &Value) -> Result<Value, Error> {
        let args = match &self.formals {
            Formals::Singleton(_) => eval(env, args)?,
            Formals::Positional(_) => {
                let array = Value::as_array(args)?;
                Value::Array(eval_array(env, &array)?)
            }
            Formals::Named(_) => {
                let object = Value::as_object(args)?;
                Value::Object(eval_object(env, object)?)
            }
        };
        self.call(&args)
    }

    fn call(&self, args: &Value) -> Result<Value, Error> {
        let mut variables = ObjectMap::new();
        match &self.formals {
            Formals::Singleton(name) => {
                variables.insert(name.to_string(), args.clone());
            }
            Formals::Positional(names) => {
                let values = Value::as_array(args)?;
                if names.len() != values.len() {
                    return Err(Error::ArgumentCountMismatch(names.len(), values.len()));
                }
                for (name, actual) in names.iter().zip(values.iter()) {
                    variables.insert(name.to_string(), actual.clone());
                }
            }
            Formals::Named(names) => {
                let values = Value::as_object(args)?;
                for name in names.iter() {
                    let actual = values
                        .get(name.as_ref())
                        .ok_or_else(|| Error::MissingNamedArgument(name.clone()))?
                        .clone();
                    variables.insert(name.to_string(), actual);
                }
            }
        };
        let env = Env::new(variables, Some(self.env.clone()));
        eval(&env, &self.body)
    }
}

pub fn eval_array(env: &Arc<Env>, array: &Vec<Value>) -> Result<Arc<Vec<Value>>, Error> {
    Ok(Arc::new(
        array
            .iter()
            .map(|value| eval(env, value))
            .collect::<Result<Vec<Value>, Error>>()?,
    ))
}

pub fn eval_object(env: &Arc<Env>, object: &Object) -> Result<Object, Error> {
    Ok(Arc::new(
        object
            .iter()
            .map(|(name, value)| {
                let value = eval(env, value)?;
                Ok((name.clone(), value))
            })
            .collect::<Result<ObjectMap, Error>>()?,
    ))
}

pub fn eval(env: &Arc<Env>, value: &Value) -> Result<Value, Error> {
    Ok(match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Function(_) => {
            value.clone()
        }
        Value::Array(values) => Value::Array(eval_array(env, values)?),
        Value::Object(object) => {
            let op = get_op(object)?;
            let func = Value::as_function(env.lookup(&op.name)?)?;
            func.eval(env, object, &op.args)?
        }
    })
}

pub fn get_key<'a>(object: &'a Object, key: &str) -> Result<&'a Value, Error> {
    object
        .get(key)
        .ok_or_else(|| Error::UnknownKey(key.to_string()))
}

pub fn get_index(array: &Vec<Value>, index: usize) -> Result<&Value, Error> {
    array
        .get(index)
        .ok_or_else(|| Error::InvalidIndex(index, array.len()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    name: String,
    args: Value,
}

fn get_op(object: &Object) -> Result<Op, Error> {
    let ops: Vec<Op> = object
        .iter()
        .filter_map(|(key, value)| {
            if key.len() == 1 || !key.starts_with("+") {
                Some(Op {
                    name: key.clone(),
                    args: value.clone(),
                })
            } else {
                None
            }
        })
        .collect();
    if ops.is_empty() {
        return Err(Error::MissingOperation);
    }
    if ops.len() > 1 {
        return Err(Error::AmbiguousOperation(ops));
    }
    Ok(ops.into_iter().next().unwrap())
}
