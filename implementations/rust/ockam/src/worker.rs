use alloc::rc::Rc;
use core::cell::RefCell;

#[derive(Clone)]
pub struct WorkerContext {}

pub trait Worker<T> {
    fn handle(&self, _message: T, _context: &mut WorkerContext) -> crate::Result<bool> {
        unimplemented!()
    }

    fn starting(&mut self, _context: &mut WorkerContext) -> crate::Result<bool> {
        Ok(true)
    }

    fn stopping(&mut self, _context: &mut WorkerContext) -> crate::Result<bool> {
        Ok(true)
    }
}

struct ClosureWorker<T> {
    message_handler: Option<Rc<RefCell<dyn FnMut(&T, &mut WorkerContext)>>>,
}

impl<T> Worker<T> for ClosureWorker<T> {
    fn handle(&self, message: T, context: &mut WorkerContext) -> crate::Result<bool> {
        if let Some(handler) = self.message_handler.clone() {
            let mut h = handler.borrow_mut();
            h(&message, context);
            Ok(true)
        } else {
            Err(crate::Error::WorkerRuntime)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::worker::{ClosureWorker, Worker, WorkerContext};
    use alloc::rc::Rc;
    use core::cell::RefCell;

    struct Thing {}

    #[test]
    fn worker() {
        let work = Rc::new(RefCell::new(
            |_message: &Thing, _context: &mut WorkerContext| {},
        ));

        let worker = ClosureWorker {
            message_handler: Some(work),
        };
        let mut context = WorkerContext {};

        worker.handle(Thing {}, &mut context).unwrap();
    }
}
