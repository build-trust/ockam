#[macro_export]
macro_rules! error {
    ($($x:expr),*) => {{
        $(
            _ = $x;
        )*
    }};
}

#[macro_export]
macro_rules! warn {
    ($($x:expr),*) => {{
        $(
            _ = $x;
        )*
    }};
}

#[macro_export]
macro_rules! debug {
    ($($x:expr),*) => {{
        $(
            _ = $x;
        )*
    }};
}

#[macro_export]
macro_rules! info {
    ($($x:expr),*) => {{
        $(
            _ = $x;
        )*
    }};
}

#[macro_export]
macro_rules! trace {
    ($($x:expr),*) => {{
        $(
            _ = $x;
        )*
    }};
}
