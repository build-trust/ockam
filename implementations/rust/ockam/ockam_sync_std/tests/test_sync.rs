extern crate alloc;
use tokio::runtime::Builder;
use ockam_sync_std::{ockam_lock_acquire,ockam_lock_new,ockam_trait};
use alloc::sync::Arc;
use tokio::sync::Mutex;

pub trait MyTrait {
    fn display(&mut self);
}

pub struct MyStruct {
    d: String,
}

impl MyTrait for MyStruct {
    fn display(&mut self) {
        println!("d: {}", self.d);
    }
}

pub async fn take_my_send_static_trait(t: ockam_trait!(dyn MyTrait + Send + 'static)) {
    let mut t = ockam_lock_acquire!(t);
    t.display();
}

pub async fn take_my_send_trait(t: ockam_trait!(dyn MyTrait + Send)) {
    let mut t = ockam_lock_acquire!(t);
    t.display();
}

pub async fn match_static(t: ockam_trait!(dyn MyTrait + 'static)) {
    let mut t = ockam_lock_acquire!(t);
    t.display();
}

pub fn main() {
    let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

    runtime.block_on(
        async {
            let s = MyStruct { d: String::from("hello + Send + 'static") };
            let t = ockam_lock_new!(dyn MyTrait + Send + 'static, s);
            take_my_send_static_trait(t).await;

            let s = MyStruct { d: String::from("hello + Send") };
            let t = ockam_lock_new!(dyn MyTrait + Send, s);
            take_my_send_trait(t).await;

            let s = MyStruct { d: String::from("hello + 'static") };
            let t = ockam_lock_new!(dyn MyTrait + 'static, s);
            match_static(t).await;
        }
    );
}
