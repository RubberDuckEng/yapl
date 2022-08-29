use lazy_static::lazy_static;
use serde_json;
use std::sync::Arc;

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
pub type NativeFunction = fn(&Map<String, Arc<Value>>) -> Result<Arc<Value>, Error>;

// TODO: Use a smarter handle than Arc to store null, bool, number, and string
// without needing a heap allocation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Arc<Value>>),
    Object(Map<String, Arc<Value>>),
    Function(Function),
}

impl Value {
    pub fn null() -> Arc<Value> {
        lazy_static! {
            static ref NULL: Arc<Value> = Arc::new(Value::Null);
        }
        NULL.clone()
    }

    pub fn empty_map() -> Arc<Value> {
        lazy_static! {
            static ref EMPTY: Arc<Value> = Arc::new(Value::Object(Map::new()));
        }
        EMPTY.clone()
    }

    fn as_string(value: &Arc<Value>) -> Result<&str, Error> {
        match value.as_ref() {
            Value::String(value) => Ok(value),
            _ => Err(Error::TypeError),
        }
    }

    fn as_function(value: &Arc<Value>) -> Result<&Function, Error> {
        match value.as_ref() {
            Value::Function(value) => Ok(value),
            _ => Err(Error::TypeError),
        }
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
    pub fn call(&self, args: &Arc<Value>) -> Result<Arc<Value>, Error> {
        if let Value::Object(args) = args.as_ref() {
            match &self.body {
                FunctionBody::Native(native) => native(args),
            }
        } else {
            Err(Error::TypeError)
        }
    }
}

fn from_serde(value: serde_json::Value) -> Arc<Value> {
    Arc::new(match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => Value::Number(value),
        serde_json::Value::String(value) => Value::String(value),
        serde_json::Value::Array(values) => {
            Value::Array(values.into_iter().map(|value| from_serde(value)).collect())
        }
        serde_json::Value::Object(value) => Value::Object(Map::from_iter(
            value
                .into_iter()
                .map(|(key, value)| (key, from_serde(value))),
        )),
    })
}

fn to_serde(value: &Arc<Value>) -> serde_json::Value {
    match value.as_ref() {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(value.clone()),
        Value::Number(value) => serde_json::Value::Number(value.clone()),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(|value| to_serde(value)).collect())
        }
        Value::Object(value) => serde_json::Value::Object(serde_json::map::Map::from_iter(
            value
                .into_iter()
                .map(|(key, value)| (key.clone(), to_serde(value))),
        )),
        Value::Function(_) => serde_json::Value::String("#function".to_string()),
    }
}

pub fn parse(json: &str) -> Result<Arc<Value>, Error> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::Parse)?;
    Ok(from_serde(value))
}

pub fn serialize(value: &Arc<Value>) -> Result<String, Error> {
    // TODO: Avoid cloning the entire value just to serialize it.
    let value: serde_json::Value = to_serde(value);
    serde_json::to_string(&value).map_err(|_| Error::Serializeation)
}

pub struct Environment {
    pub variables: Map<String, Arc<Value>>,
}

impl Environment {
    pub fn builtin() -> Environment {
        let mut env = Environment {
            variables: Map::new(),
        };
        env.bind_native("println", builtins::println);
        env
    }

    pub fn lookup(&self, name: &str) -> Result<&Arc<Value>, Error> {
        self.variables
            .get(name)
            .ok_or(Error::UnknownVariable(name.to_string()))
    }

    pub fn bind_native(&mut self, name: &str, function: NativeFunction) {
        self.variables.insert(
            name.to_string(),
            Arc::new(Value::Function(Function {
                body: FunctionBody::Native(function),
            })),
        );
    }
}

pub fn eval(env: &Environment, value: &Arc<Value>) -> Result<Arc<Value>, Error> {
    Ok(match value.as_ref() {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Function(_) => {
            value.clone()
        }
        Value::Array(values) => Arc::new(Value::Array(
            values
                .into_iter()
                .map(|value| eval(env, value))
                .collect::<Result<Vec<Arc<Value>>, Error>>()?,
        )),
        Value::Object(object) => {
            // TODO: This doesn't evaluate everything properly yet.
            let op = object.get("op").ok_or(Error::MissingOperation)?;
            let func = Value::as_function(env.lookup(Value::as_string(op)?)?)?;
            let empty_map = Value::empty_map();
            let args = object.get("args").unwrap_or(&empty_map);
            func.call(args)?
        }
    })
}
