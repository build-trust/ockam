use zeroize::Zeroize;

pub trait ErrorVault: Zeroize {
    fn error_domain() -> &'static str;
}
