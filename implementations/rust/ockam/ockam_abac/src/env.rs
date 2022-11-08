use crate::error::MergeError;
use crate::expr::Expr;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::string::String;

#[derive(Debug, Clone)]
pub struct Env {
    map: BTreeMap<String, Expr>,
    null: Expr,
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

impl Env {
    pub fn new() -> Self {
        Env {
            map: BTreeMap::new(),
            null: Expr::Null,
        }
    }

    pub fn get(&self, k: &str) -> &Expr {
        self.map.get(k).unwrap_or(&self.null)
    }

    pub fn contains(&self, k: &str) -> bool {
        self.map.contains_key(k)
    }

    pub fn put<K: Into<String>, E: Into<Expr>>(&mut self, k: K, v: E) -> &mut Self {
        self.map.insert(k.into(), v.into());
        self
    }

    pub fn del(&mut self, k: &str) {
        self.map.remove(k);
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &Expr)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn merge(&mut self, other: Env) -> Result<(), MergeError> {
        for k in other.map.keys() {
            if self.map.contains_key(k) {
                return Err(MergeError::BindingExists(k.clone()));
            }
        }
        for (k, v) in other.map.into_iter() {
            self.map.insert(k, v);
        }
        Ok(())
    }

    pub fn merge_right(&mut self, other: Env) {
        for (k, v) in other.map.into_iter() {
            self.map.insert(k, v);
        }
    }

    pub fn merge_left(&mut self, other: Env) {
        for (k, v) in other.map.into_iter() {
            self.map.entry(k).or_insert(v);
        }
    }
}
