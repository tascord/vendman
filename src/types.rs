use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Dependency {
    Dep(String),
    DepWithHash(String, String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub version: String,
    pub dependencies: HashMap<String, Dependency>,
}