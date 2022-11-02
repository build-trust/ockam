use crate::test_utils::pool::Reusable;
use assert_cmd::prelude::*;
use lazy_static::lazy_static;
use pool::Pool;
use std::process::Command;
use std::sync::Arc;

lazy_static! {
    static ref NODES_POOL: Arc<Pool<TestNode>> = init_node_pool();
}

pub(crate) struct TestNode {
    name: String,
    init: bool,
}

impl TestNode {
    fn new(name: String) -> Self {
        Self { name, init: false }
    }

    pub(crate) fn name(&self) -> &str {
        assert!(self.init);
        &self.name
    }
}

fn init_node_pool() -> Arc<Pool<TestNode>> {
    // In reverse order because the elements will be popped from the pool's stack.
    // The names are fixed to avoid cluttering the local dev environment with
    // randomly named nodes.
    let mut node_names = vec![];
    for i in (0..16).rev() {
        node_names.push(TestNode::new(format!("test{}", i + 1)));
    }
    Arc::new(Pool::from_iter(node_names))
}

pub(crate) fn get_test_node() -> Reusable<'static, TestNode> {
    loop {
        if let Some(mut node) = NODES_POOL.try_pull() {
            // Nodes are initialized lazily to avoid unnecessary overhead.
            if !node.init {
                let mut cmd = Command::cargo_bin("ockam").unwrap();
                cmd.args(&["node", "delete", &node.name, "-f"]);
                cmd.assert().success();
                std::thread::sleep(std::time::Duration::from_millis(250));

                let mut cmd = Command::cargo_bin("ockam").unwrap();
                cmd.args(&["node", "create", &node.name]);
                if !cmd.output().is_ok() {
                    node.detach();
                    continue;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));

                node.init = true;
            }
            return node;
        }
    }
}

#[allow(dead_code)]
mod pool {
    // Got from https://github.com/CJP10/object-pool, licenses: MIT/Apache-2.0
    // crates.io version is outdated

    use std::iter::FromIterator;
    use std::mem::{forget, ManuallyDrop};
    use std::ops::{Deref, DerefMut};
    use std::sync::Mutex;

    pub type Stack<T> = Vec<T>;

    pub struct Pool<T> {
        objects: Mutex<Stack<T>>,
    }

    impl<T> Pool<T> {
        #[inline]
        pub fn new<F>(cap: usize, init: F) -> Pool<T>
        where
            F: Fn() -> T,
        {
            Pool {
                objects: Mutex::new((0..cap).into_iter().map(|_| init()).collect()),
            }
        }

        #[inline]
        pub fn from_vec(v: Vec<T>) -> Pool<T> {
            Pool {
                objects: Mutex::new(v),
            }
        }

        #[inline]
        pub fn len(&self) -> usize {
            self.objects.lock().unwrap().len()
        }

        #[inline]
        pub fn is_empty(&self) -> bool {
            self.objects.lock().unwrap().is_empty()
        }

        #[inline]
        pub fn try_pull(&self) -> Option<Reusable<T>> {
            self.objects
                .lock()
                .unwrap()
                .pop()
                .map(|data| Reusable::new(self, data))
        }

        #[inline]
        pub fn pull<F: Fn() -> T>(&self, fallback: F) -> Reusable<T> {
            self.try_pull()
                .unwrap_or_else(|| Reusable::new(self, fallback()))
        }

        #[inline]
        pub fn attach(&self, t: T) {
            self.objects.lock().unwrap().push(t)
        }
    }

    impl<T> FromIterator<T> for Pool<T> {
        fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
            Self {
                objects: Mutex::new(iter.into_iter().collect()),
            }
        }
    }

    pub struct Reusable<'a, T> {
        pool: &'a Pool<T>,
        data: ManuallyDrop<T>,
    }

    impl<'a, T> Reusable<'a, T> {
        #[inline]
        pub fn new(pool: &'a Pool<T>, t: T) -> Self {
            Self {
                pool,
                data: ManuallyDrop::new(t),
            }
        }

        #[inline]
        pub fn detach(mut self) -> (&'a Pool<T>, T) {
            let ret = unsafe { (self.pool, self.take()) };
            forget(self);
            ret
        }

        unsafe fn take(&mut self) -> T {
            ManuallyDrop::take(&mut self.data)
        }
    }

    impl<'a, T> Deref for Reusable<'a, T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    impl<'a, T> DerefMut for Reusable<'a, T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    impl<'a, T> Drop for Reusable<'a, T> {
        #[inline]
        fn drop(&mut self) {
            unsafe { self.pool.attach(self.take()) }
        }
    }
}
