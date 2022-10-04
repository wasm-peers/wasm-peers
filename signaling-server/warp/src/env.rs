use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Env {
    pub example_bool: bool,
    pub example_list: Vec<String>,
}

/// Access to parsed environment variables.
pub static ENV: Lazy<Env> = Lazy::new(|| envy::from_env().expect("some env vars missing"));
