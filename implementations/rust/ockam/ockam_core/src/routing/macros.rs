#[macro_export]
/// Create a route
macro_rules! route {
    () => (
        crate::Route::from(crate::Route::new())
    );
    ($($x:expr),+) => ({
        let mut r = crate::Route::new();
        $(r = r.append($x);)*
        crate::Route::from(r)
    });
}

#[cfg(test)]
mod tests {
    use crate::Address;
    use rand::random;

    #[test]
    fn test1() {
        let _route = route![];
    }

    #[test]
    fn test2() {
        use crate::std::string::ToString;
        let address: Address = random();
        let _route = route!["str", "STR2", "STR3".to_string(), address];
    }
}
