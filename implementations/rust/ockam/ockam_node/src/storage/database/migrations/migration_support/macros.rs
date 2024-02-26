/// Create a migrator given a path to sql script
#[macro_export]
macro_rules! migrate {
    ($dir:literal) => {{
        ockam_macros::migrate!($dir)
    }};
}
