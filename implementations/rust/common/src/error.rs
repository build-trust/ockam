pub trait ErrorKind {
    const ERROR_INTERFACE: usize;
    fn to_usize(&self) -> usize;
}
