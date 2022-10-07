use crate::error::EvalError;
use crate::expr::Expr;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::{String, ToString};

#[derive(Debug, Clone, Default)]
pub struct Env(BTreeMap<String, Expr>);

impl Env {
    pub fn new() -> Self {
        Env(BTreeMap::new())
    }

    pub fn get(&self, k: &str) -> Result<&Expr, EvalError> {
        self.0
            .get(k)
            .ok_or_else(|| EvalError::Unbound(k.to_string()))
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
}
