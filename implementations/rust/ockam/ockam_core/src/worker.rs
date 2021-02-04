use crate::Result;

pub trait Worker: Send + 'static {
    type Context;

    fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }
}

pub trait Handler<M>: Worker {
    fn handle(&mut self, _context: &mut Self::Context, _message: M) -> Result<()> {
        Ok(())
    }
}
