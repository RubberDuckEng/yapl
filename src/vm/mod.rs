use lazy_static::lazy_static;
use serde_json;

mod builtins;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EvalError,
    Parse,
    Serializeation,
    MissingOperation,
    TypeError,
    UnknownVariable(String),
}

pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type Number = serde_json::Number;
pub type NativeFunction = fn(&Map<String, Value>) -> Result<Value, Error>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
    Function(Function),
}

impl Value {
    pub fn empty_map() -> &'static Self {
        lazy_static! {
            static ref EMPTY: Value = Value::Object(Map::new());
        }
        &EMPTY
    }
}

#[derive(Clone)]
enum FunctionBody {
    Native(NativeFunction),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    body: FunctionBody,
}

impl Function {
    pub fn call(&self, args: &Value) -> Result<Value, Error> {
        if let Value::Object(args) = args {
            match &self.body {
                FunctionBody::Native(native) => native(args),
            }
        } else {
            Err(Error::TypeError)
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Value {
        match value {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(value) => Value::Bool(value),
            serde_json::Value::Number(value) => Value::Number(value),
            serde_json::Value::String(value) => Value::String(value),
            serde_json::Value::Array(values) => {
                Value::Array(values.into_iter().map(|value| value.into()).collect())
            }
            serde_json::Value::Object(value) => Value::Object(Map::from_iter(
                value.into_iter().map(|(key, value)| (key, value.into())),
            )),
        }
    }
}

impl From<Value> for serde_json::Value {
    fn from(value: Value) -> serde_json::Value {
        match value {
            Value::Null => serde_json::Value::Null,
            Value::Bool(value) => serde_json::Value::Bool(value),
            Value::Number(value) => serde_json::Value::Number(value),
            Value::String(value) => serde_json::Value::String(value),
            Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(|value| value.into()).collect())
            }
            Value::Object(value) => serde_json::Value::Object(serde_json::map::Map::from_iter(
                value.into_iter().map(|(key, value)| (key, value.into())),
            )),
            Value::Function(_) => serde_json::Value::String("#function".to_string()),
        }
    }
}

pub fn parse(json: &str) -> Result<Value, Error> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::Parse)?;
    Ok(value.into())
}

pub fn serialize(value: &Value) -> Result<String, Error> {
    // TODO: Avoid cloning the entire value just to serialize it.
    let value: serde_json::Value = value.clone().into();
    serde_json::to_string(&value).map_err(|_| Error::Serializeation)
}

pub struct Environment {
    pub variables: Map<String, Value>,
}

impl Environment {
    pub fn builtin() -> Environment {
        let mut env = Environment {
            variables: Map::new(),
        };
        env.bind_native("println", builtins::println);
        env
    }

    pub fn lookup(&self, name: &str) -> Result<&Value, Error> {
        self.variables
            .get(name)
            .ok_or(Error::UnknownVariable(name.to_string()))
    }

    pub fn bind_native(&mut self, name: &str, function: NativeFunction) {
        self.variables.insert(
            name.to_string(),
            Value::Function(Function {
                body: FunctionBody::Native(function),
            }),
        );
    }
}

fn as_string(value: &Value) -> Result<&str, Error> {
    match value {
        Value::String(value) => Ok(value),
        _ => Err(Error::TypeError),
    }
}

fn as_func(value: &Value) -> Result<&Function, Error> {
    match value {
        Value::Function(value) => Ok(value),
        _ => Err(Error::TypeError),
    }
}

pub fn eval(env: &Environment, value: Value) -> Result<Value, Error> {
    Ok(match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Function(_) => {
            value
        }
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| eval(env, value))
                .collect::<Result<Vec<Value>, Error>>()?,
        ),
        Value::Object(object) => {
            let op = object.get("op").ok_or(Error::MissingOperation)?;
            let func = as_func(env.lookup(as_string(op)?)?)?;
            let empty_map = Value::empty_map();
            let args = object.get("args").unwrap_or(empty_map);
            func.call(args)?
        }
    })
}
