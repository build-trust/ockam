/// Creates a [`Route`] containing the arguments.
///
/// `route!` allows `Route`s to be defined with the same syntax as array expressions.
///
/// ```
/// # use ockam_core::{Route, route, Address};
/// # use rand::random;
/// let address4: Address = random();
/// let route = route!["address1", "address2", "address3".to_string(), address4];
/// ```
///
/// [`Route`]: crate::Route
#[macro_export]
macro_rules! route {
    ($($x:expr),* $(,)?) => ({
        #[allow(unused_mut)]
        let mut r = $crate::Route::new();
        $(r = r.append($x);)*
        $crate::Route::from(r)
    });
}

#[cfg(test)]
mod tests {
    use crate::compat::rand::random;
    use crate::Address;

    #[test]
    fn test1() {
        let _route = route![];
    }

    #[test]
    fn test2() {
        use crate::compat::string::ToString;
        let address: Address = random();
        let _route = route!["str", "STR2", "STR3".to_string(), address];
    }

    #[test]
    fn test3() {
        let _route = route!["str",];
    }
}
