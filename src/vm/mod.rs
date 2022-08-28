use serde_json;

mod builtins;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    EvalError,
    ParseError,
    SerializeationError,
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
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| Error::ParseError)?;
    Ok(value.into())
}

pub fn serialize(value: &Value) -> Result<String, Error> {
    // TODO: Avoid cloning the entire value just to serialize it.
    let value: serde_json::Value = value.clone().into();
    serde_json::to_string(&value).map_err(|_| Error::SerializeationError)
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

    pub fn bind_native(&mut self, name: &str, function: NativeFunction) {
        self.variables.insert(
            name.to_string(),
            Value::Function(Function {
                body: FunctionBody::Native(function),
            }),
        );
    }
}

pub fn eval(_env: &Environment, _value: &Value) -> Result<Value, Error> {
    Err(Error::EvalError)
}
