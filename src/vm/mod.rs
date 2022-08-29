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
    InvalidOperation,
    UndefinedVariable(String),
    UnknownKey(String),
}

pub type Object = Map<String, Arc<Value>>;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type Number = serde_json::Number;
pub type NativeFunction = fn(&Object) -> Result<Arc<Value>, Error>;
pub type NativeSpecialForm = fn(&Arc<Environment>, &Object) -> Result<Arc<Value>, Error>;

// TODO: Use a smarter handle than Arc to store null, bool, number, and string
// without needing a heap allocation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Arc<Value>>),
    Object(Object),
    Function(Arc<Function>),
}

impl Value {
    pub fn null() -> Arc<Value> {
        lazy_static! {
            static ref NULL: Arc<Value> = Arc::new(Value::Null);
        }
        NULL.clone()
    }

    pub fn empty_object() -> Arc<Value> {
        lazy_static! {
            static ref EMPTY: Arc<Value> = Arc::new(Value::Object(Object::new()));
        }
        EMPTY.clone()
    }

    pub fn as_string(value: &Arc<Value>) -> Result<&str, Error> {
        match value.as_ref() {
            Value::String(value) => Ok(value),
            _ => Err(Error::TypeError),
        }
    }

    pub fn as_function(value: &Arc<Value>) -> Result<&Function, Error> {
        match value.as_ref() {
            Value::Function(value) => Ok(value),
            _ => Err(Error::TypeError),
        }
    }

    pub fn as_object(value: &Arc<Value>) -> Result<&Object, Error> {
        match value.as_ref() {
            Value::Object(value) => Ok(value),
            _ => Err(Error::TypeError),
        }
    }

    pub fn as_array(value: &Arc<Value>) -> Result<&Vec<Arc<Value>>, Error> {
        match value.as_ref() {
            Value::Array(values) => Ok(values),
            _ => Err(Error::TypeError),
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
    pub fn call(&self, env: &Arc<Environment>, object: &Object) -> Result<Arc<Value>, Error> {
        let get_args = || {
            let empty_object = Value::empty_object();
            eval_object(
                env,
                Value::as_object(get_key(object, "args").unwrap_or(&empty_object))?,
            )
        };

        match &self.body {
            FunctionBody::Native(native) => {
                let args = get_args()?;
                native(Value::as_object(&args)?)
            }
            FunctionBody::Lambda(lambda) => {
                let args = get_args()?;
                lambda.call(Value::as_object(&args)?)
            }
            FunctionBody::NativeSpecialForm(special_form) => special_form(env, object),
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

#[derive(Debug, PartialEq, Eq)]
pub struct Environment {
    pub variables: Object,
    pub parent: Option<Arc<Environment>>,
}

impl Environment {
    pub fn builtin() -> Arc<Environment> {
        let mut env = Environment {
            variables: Map::new(),
            parent: None,
        };
        env.bind_native_function("println", builtins::println);
        env.bind_native_special_form("lambda", builtins::lambda);
        env.bind_native_special_form("lookup", builtins::lookup);
        Arc::new(env)
    }

    pub fn lookup(&self, name: &str) -> Result<&Arc<Value>, Error> {
        if let Some(value) = self.variables.get(name) {
            Ok(value)
        } else if let Some(parent) = &self.parent {
            parent.lookup(name)
        } else {
            Err(Error::UndefinedVariable(name.to_string()))
        }
    }

    pub fn bind_native_function(&mut self, name: &str, function: NativeFunction) {
        self.variables.insert(
            name.to_string(),
            Arc::new(Value::Function(Arc::new(Function {
                body: FunctionBody::Native(function),
            }))),
        );
    }

    pub fn bind_native_special_form(&mut self, name: &str, special_form: NativeSpecialForm) {
        self.variables.insert(
            name.to_string(),
            Arc::new(Value::Function(Arc::new(Function {
                body: FunctionBody::NativeSpecialForm(special_form),
            }))),
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Lambda {
    env: Arc<Environment>,
    formals: Vec<String>,
    body: Arc<Value>,
}

impl Lambda {
    pub fn call(&self, args: &Object) -> Result<Arc<Value>, Error> {
        let mut variables = Map::new();
        for name in &self.formals {
            variables.insert(name.clone(), args[name].clone());
        }
        let env = Arc::new(Environment {
            variables,
            parent: Some(self.env.clone()),
        });
        let result = eval(&env, &self.body)?;
        Ok(result)
    }
}

fn eval_object(env: &Arc<Environment>, object: &Object) -> Result<Arc<Value>, Error> {
    let evaluted_object = Object::from_iter(
        object
            .into_iter()
            .map(|(key, value)| {
                let value = eval(env, value)?;
                Ok((key.clone(), value))
            })
            .collect::<Result<Vec<(String, Arc<Value>)>, Error>>()?,
    );
    Ok(Arc::new(Value::Object(evaluted_object)))
}

pub fn eval(env: &Arc<Environment>, value: &Arc<Value>) -> Result<Arc<Value>, Error> {
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
            let op = get_key(object, "op")?;
            let func = match op.as_ref() {
                Value::String(name) => env.lookup(&name)?.clone(),
                _ => eval(env, op)?,
            };
            Value::as_function(&func)?.call(env, object)?
        }
    })
}

pub fn get_key<'a>(object: &'a Object, key: &str) -> Result<&'a Arc<Value>, Error> {
    object.get(key).ok_or_else(|| {
        // let me break here
        Error::UnknownKey(key.to_string())
    })
}
