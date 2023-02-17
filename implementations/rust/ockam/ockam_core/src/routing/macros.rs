/// Creates a new [`Route`] from a comma-delimited list of [`Address`]es.
///
/// The `route!` macro allows a `Route` to be defined with the same
/// syntax as array expressions:
///
/// ```
/// # use ockam_core::{Route, route, Address};
/// # use ockam_core::compat::rand::random;
/// let address4: Address = random();
/// let route = route!["address1", "address2", "address3".to_string(), address4];
/// ```
///
/// [`Address`]: Into<Route>
/// [`Route`]: crate::Route
#[macro_export]
macro_rules! route {
    ($($x:expr),* $(,)?) => ({
        #[allow(unused_mut)]
        let mut r = $crate::Route::new();
        $(r = r.append_route($x.into());)*
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
        let route1 = route!["str", "STR2", "STR3".to_string(), address];
        let _route2 = route![route1.clone(), "str1", route1];
    }

    #[test]
    fn test3() {
        let _route = route!["str",];
    }

    #[test]
    fn test4() {
        let route1 = route!["s1", "s2"];
        let route2 = route!["s4"];
        let route3 = route![route1, "s3", route2];
        assert_eq!(route3, route!["s1", "s2", "s3", "s4"])
    }
}
