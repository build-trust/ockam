pub trait Runner: Send + 'static {
    type Context: Send + 'static;

    fn set_ctx(&mut self, ctx: Self::Context);
}
