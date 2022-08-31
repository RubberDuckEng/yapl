use lazy_static::lazy_static;
use serde_json;
use serde_yaml;
use std::sync::Arc;

mod builtins;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Parse,
    Serializeation,
    MissingOperation,
    AmbiguousOperation(Vec<Op>),
    Type(String),
    InvalidOperation,
    UndefinedSymbol(String),
    UnknownKey(String),
    InvalidIndex(usize, usize),
    ArgumentCountMismatch(usize, usize),
    MissingNamedArgument(String),
    IO,
}

impl Error {
    fn new_type(expected: &str, actual: &Arc<Value>) -> Error {
        Error::Type(format!("Expected {}, got {}", expected, actual.type_of()))
    }
}

pub type Object = Map<String, Arc<Value>>;
pub type Map<K, V> = std::collections::HashMap<K, V>;
pub type Number = serde_json::Number;
pub type NativeFunction = fn(&Arc<Environment>, &Arc<Value>) -> Result<Arc<Value>, Error>;
pub type NativeSpecialForm =
    fn(&Arc<Environment>, &Object, &Arc<Value>) -> Result<Arc<Value>, Error>;

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

    pub fn empty_object() -> Arc<Value> {
        lazy_static! {
            static ref EMPTY: Arc<Value> = Arc::new(Value::Object(Object::new()));
        }
        EMPTY.clone()
    }

    pub fn as_bool(value: &Arc<Value>) -> Result<bool, Error> {
        match value.as_ref() {
            Value::Bool(value) => Ok(*value),
            _ => Err(Error::new_type("bool", value)),
        }
    }

    pub fn as_string(value: &Arc<Value>) -> Result<&str, Error> {
        match value.as_ref() {
            Value::String(value) => Ok(value),
            _ => Err(Error::new_type("string", value)),
        }
    }

    pub fn as_function(value: &Arc<Value>) -> Result<&Function, Error> {
        match value.as_ref() {
            Value::Function(value) => Ok(value),
            _ => Err(Error::new_type("function", value)),
        }
    }

    pub fn as_object(value: &Arc<Value>) -> Result<&Object, Error> {
        match value.as_ref() {
            Value::Object(value) => Ok(value),
            _ => Err(Error::new_type("object", value)),
        }
    }

    pub fn as_array(value: &Arc<Value>) -> Result<&Vec<Arc<Value>>, Error> {
        match value.as_ref() {
            Value::Array(values) => Ok(values),
            _ => Err(Error::new_type("array", value)),
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
    fn eval_args(&self, env: &Arc<Environment>, args: &Arc<Value>) -> Result<Arc<Value>, Error> {
        match &self.body {
            FunctionBody::Native(_) => {
                // TODO: This isn't quite right. We need to be able to know whether
                // the native function takes an object, an array, or a singleton value.
                eval(env, args)
            }
            FunctionBody::Lambda(lambda) => lambda.eval_args(env, args),
            FunctionBody::NativeSpecialForm(_) => Ok(args.clone()),
        }
    }

    pub fn call(
        &self,
        env: &Arc<Environment>,
        object: &Object,
        args: &Arc<Value>,
    ) -> Result<Arc<Value>, Error> {
        match &self.body {
            FunctionBody::Native(native) => native(env, args),
            FunctionBody::Lambda(lambda) => lambda.call(args),
            FunctionBody::NativeSpecialForm(special_form) => special_form(env, object, args),
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
    let value: serde_json::Value = serde_yaml::from_str(json).map_err(|_| Error::Parse)?;
    Ok(from_serde(value))
}

pub fn serialize(value: &Arc<Value>) -> Result<String, Error> {
    // TODO: Avoid cloning the entire value just to serialize it.
    let value: serde_json::Value = to_serde(value);
    serde_json::to_string(&value).map_err(|_| Error::Serializeation)
}

pub const FILE_SYMBOL: &str = "__file__";

#[derive(Debug, PartialEq, Eq)]
pub struct Environment {
    pub variables: Object,
    pub parent: Option<Arc<Environment>>,
}

impl Environment {
    pub fn builtin(path: String) -> Arc<Environment> {
        let mut env = Environment {
            variables: Map::new(),
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
        env.bind_native_special_form("$", builtins::lookup);
        env.bind_native_special_form("export", builtins::export);
        env.bind_native_special_form("import", builtins::import);
        env.bind_native_special_form("lambda", builtins::lambda);
        env.bind_native_special_form("let", builtins::nonrecursive_let);
        env.bind_native_special_form("quote", builtins::quote);
        env.bind_native_special_form("if", builtins::if_func);
        Arc::new(env)
    }

    pub fn new(variables: Object, parent: Option<Arc<Environment>>) -> Arc<Environment> {
        Arc::new(Environment { variables, parent })
    }

    pub fn lookup(&self, name: &str) -> Result<&Arc<Value>, Error> {
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
            .insert(name.to_string(), Arc::new(Value::String(string)));
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
pub enum Formals {
    Singleton(String),
    Positional(Vec<String>),
    Named(Vec<String>),
}

#[derive(Debug, PartialEq, Eq)]
struct Lambda {
    env: Arc<Environment>,
    formals: Formals,
    body: Arc<Value>,
}

impl Lambda {
    fn eval_args(&self, env: &Arc<Environment>, args: &Arc<Value>) -> Result<Arc<Value>, Error> {
        Ok(match &self.formals {
            Formals::Singleton(_) => eval(env, args)?,
            Formals::Positional(_) => {
                let array = Value::as_array(args)?;
                Arc::new(Value::Array(eval_array(env, &array)?))
            }
            Formals::Named(_) => {
                let object = Value::as_object(args)?;
                Arc::new(Value::Object(eval_object(env, object)?))
            }
        })
    }

    fn call(&self, args: &Arc<Value>) -> Result<Arc<Value>, Error> {
        let mut variables = Map::new();
        match &self.formals {
            Formals::Singleton(name) => {
                variables.insert(name.clone(), args.clone());
            }
            Formals::Positional(names) => {
                let values = Value::as_array(args)?;
                if names.len() != values.len() {
                    return Err(Error::ArgumentCountMismatch(names.len(), values.len()));
                }
                for (name, actual) in names.iter().zip(values.iter()) {
                    variables.insert(name.clone(), actual.clone());
                }
            }
            Formals::Named(names) => {
                let values = Value::as_object(args)?;
                for name in names.iter() {
                    let actual = values
                        .get(name)
                        .ok_or_else(|| Error::MissingNamedArgument(name.clone()))?
                        .clone();
                    variables.insert(name.clone(), actual);
                }
            }
        };
        let env = Environment::new(variables, Some(self.env.clone()));
        eval(&env, &self.body)
    }
}

pub fn eval_array(
    env: &Arc<Environment>,
    array: &Vec<Arc<Value>>,
) -> Result<Vec<Arc<Value>>, Error> {
    array
        .iter()
        .map(|value| eval(env, value))
        .collect::<Result<Vec<Arc<Value>>, Error>>()
}

pub fn eval_object(env: &Arc<Environment>, object: &Object) -> Result<Object, Error> {
    object
        .iter()
        .map(|(name, value)| {
            let value = eval(env, value)?;
            Ok((name.clone(), value))
        })
        .collect::<Result<Object, Error>>()
}

pub fn eval(env: &Arc<Environment>, value: &Arc<Value>) -> Result<Arc<Value>, Error> {
    Ok(match value.as_ref() {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Function(_) => {
            value.clone()
        }
        Value::Array(values) => Arc::new(Value::Array(eval_array(env, values)?)),
        Value::Object(object) => {
            let op = get_op(object)?;
            let func = Value::as_function(env.lookup(&op.name)?)?;
            let args = func.eval_args(env, &op.args)?;
            func.call(env, object, &args)?
        }
    })
}

pub fn get_key<'a>(object: &'a Object, key: &str) -> Result<&'a Arc<Value>, Error> {
    object
        .get(key)
        .ok_or_else(|| Error::UnknownKey(key.to_string()))
}

pub fn get_index(array: &Vec<Arc<Value>>, index: usize) -> Result<&Arc<Value>, Error> {
    array
        .get(index)
        .ok_or_else(|| Error::InvalidIndex(index, array.len()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Op {
    name: String,
    args: Arc<Value>,
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
