use crate::error::{EvalError, MergeError};
use crate::expr::Expr;
use core::fmt::{Display, Formatter};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::format;
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::vec::vec;

#[derive(Debug, Clone, Default)]
pub struct Env(BTreeMap<String, Expr>);

impl Display for Env {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut values = vec![];
        for (key, value) in self.0.clone() {
            values.push(format!("{key}={value:?}"));
        }
        f.debug_struct("Env")
            .field("values", &values.join(","))
            .finish()
    }
}

impl Env {
    pub fn new() -> Self {
        Env(BTreeMap::new())
    }
    pub fn new_from(v: BTreeMap<&str, Expr>) -> Self {
        Env(v.into_iter().map(|(k, e)| (k.to_string(), e)).collect())
    }

    pub fn get(&self, k: &str) -> Result<&Expr, EvalError> {
        self.0
            .get(k)
            .ok_or_else(|| EvalError::Unbound(k.to_string()))
    }

    pub fn contains(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }

    pub fn put<K: Into<String>, E: Into<Expr>>(&mut self, k: K, v: E) -> &mut Self {
        self.0.insert(k.into(), v.into());
        self
    }

    pub fn del(&mut self, k: &str) {
        self.0.remove(k);
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &Expr)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn merge(&mut self, other: Env) -> Result<(), MergeError> {
        for k in other.0.keys() {
            if self.0.contains_key(k) {
                return Err(MergeError::BindingExists(k.clone()));
            }
        }
        for (k, v) in other.0.into_iter() {
            self.0.insert(k, v);
        }
        Ok(())
    }

    pub fn merge_right(&mut self, other: Env) {
        for (k, v) in other.0.into_iter() {
            self.0.insert(k, v);
        }
    }

    pub fn merge_left(&mut self, other: Env) {
        for (k, v) in other.0.into_iter() {
            self.0.entry(k).or_insert(v);
        }
    }
}
