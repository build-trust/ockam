use ockam_core::async_trait;
use parking_lot::RwLock;
//use crate::compat::sync::RwLock;
use super::{Abac, Action, Cond, Resource, Subject, Val};
use std::collections::HashMap;
//use crate::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;

#[derive(Debug, Default)]
pub struct Memory {
    /// subject maps to a set of key-value attributes
    subjects: HashMap<Subject, HashMap<String, Val>>,
    /// policies map a resource to a set of actions subject to conditions
    policies: HashMap<Resource, HashMap<Action, Cond>>,
}

impl Memory {
    pub fn new() -> Self {
        Memory::default()
    }

    pub fn set_subject<I>(&mut self, s: Subject, attrs: I)
    where
        I: IntoIterator<Item = (String, Val)>,
    {
        self.subjects.insert(s, attrs.into_iter().collect());
    }

    pub fn set_policy(&mut self, r: Resource, a: Action, c: Cond) {
        self.policies
            .entry(r)
            .or_insert_with(HashMap::new)
            .insert(a, c);
    }

    pub fn del_subject(&mut self, s: &Subject) {
        self.subjects.remove(s);
    }

    pub fn del_policy(&mut self, r: &Resource) {
        self.policies.remove(r);
    }

    pub fn is_authorised(&self, s: &Subject, r: &Resource, a: &Action) -> bool {
        if let Some(s) = self.subjects.get(s) {
            if let Some(c) = self.policies.get(r).and_then(|p| p.get(a)) {
                return c.apply(s);
            }
        }
        false
    }
}

#[async_trait]
impl Abac for Arc<RwLock<Memory>> {
    async fn set_subject<I>(&self, s: Subject, attrs: I)
    where
        I: IntoIterator<Item = (String, Val)> + Send + 'static,
    {
        self.write().set_subject(s, attrs)
    }

    async fn del_subject(&self, s: &Subject) {
        self.write().del_subject(s)
    }

    async fn set_policy(&self, r: Resource, a: Action, c: Cond) {
        self.write().set_policy(r, a, c)
    }

    async fn del_policy(&self, r: &Resource) {
        self.write().del_policy(r)
    }

    async fn is_authorised(&self, s: &Subject, r: &Resource, a: &Action) -> bool {
        self.read().is_authorised(s, r, a)
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Memory, Resource, Subject};
    use crate::abac::{eq, gt, int, string};

    #[test]
    fn example1() {
        let is_adult = gt("age", int(17));
        let is_john = eq("name", string("John"));
        let condition = is_adult.or(is_john);

        let read = Action::from("r");
        let resource = Resource::from("/foo/bar/baz");

        let mut mem = Memory::new();
        mem.set_policy(resource.clone(), read.clone(), condition);
        mem.set_subject(
            Subject(1),
            [
                ("name".to_string(), string("John")),
                ("age".to_string(), int(25)),
            ],
        );
        mem.set_subject(
            Subject(2),
            [
                ("name".to_string(), string("Jack")),
                ("age".to_string(), int(12)),
                ("city".to_string(), string("London")),
            ],
        );
        mem.set_subject(
            Subject(3),
            [
                ("name".to_string(), string("Bill")),
                ("age".to_string(), int(32)),
            ],
        );

        assert!(mem.is_authorised(&Subject(1), &resource, &read)); // John
        assert!(mem.is_authorised(&Subject(3), &resource, &read)); // adult
        assert!(!mem.is_authorised(&Subject(2), &resource, &read)); // not John and no adult
    }
}
