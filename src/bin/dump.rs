extern crate jsonpl;

use anyhow::Result;
use serde_json;
use serde_yaml;
use std::env;
use std::fs;
use std::process::exit;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <file>", args[0]);
        exit(1);
    }
    let path = &args[1];
    let input = fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&input)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
