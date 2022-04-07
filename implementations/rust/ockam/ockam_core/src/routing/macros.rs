/// Creates a new [`Route`] from a comma-delimited list of [`Address`]es.
///
/// The `route!` macro allows a `Route` to be defined with the same
/// syntax as array expressions:
///
/// ```
/// # use ockam_core::{Route, route, Address};
/// let address = Address::random_local();
/// let route = route![Address::local("address1"), Address::local("address2"), address];
/// ```
///
/// [`Address`]: crate::Address
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

/// Like [`route!`] but for types whose conversion to [`Address`] might fail.
///
/// ```
/// # use ockam_core::{Route, try_route, Address};
/// let address = Address::random_local();
/// let route = try_route![Address::local("address1"), Address::local("address2"), address]?;
/// # Ok::<_, ockam_core::Error>(())
/// ```
///
/// [`Address`]: crate::Address
/// [`Route`]: crate::Route
#[macro_export]
macro_rules! try_route {
    ($($x:expr),* $(,)?) => ({
        (|| {
            #[allow(unused_mut)]
            let mut r = $crate::Route::new();
            $(r = r.try_append($x)?;)*
            Ok::<_, $crate::Error>($crate::Route::from(r))
        })()
    });
}

#[cfg(test)]
mod tests {
    use crate::Address;

    #[test]
    fn test1() {
        let _route = route![];
    }

    #[test]
    fn test2() {
        use crate::compat::string::ToString;
        let address = Address::random_local();
        let _route = try_route!["str", "STR2", "STR3".to_string(), address].unwrap();
    }

    #[test]
    fn test3() {
        let _route = try_route!["str",].unwrap();
    }
}
