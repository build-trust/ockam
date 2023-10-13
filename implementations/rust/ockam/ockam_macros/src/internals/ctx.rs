use std::cell::RefCell;
use std::fmt::Display;
use std::thread;

use quote::ToTokens;

/// A type to collect errors together and format them.
///
/// Dropping this object will cause a panic. It must be consumed using `check`.
///
/// References can be shared since this type uses run-time exclusive mut checking.
#[derive(Default)]
pub(crate) struct Context {
    // The contents will be set to `None` during checking. This is so that checking can be
    // enforced.
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl Context {
    /// Create a new context object.
    ///
    /// This object contains no errors, but will still trigger a panic if it is not `check`ed.
    pub(crate) fn new() -> Self {
        Context {
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    /// Add an error to the context object with a tokenizable object.
    ///
    /// The object is used for spanning in error messages.
    pub(crate) fn error_spanned_by<A: ToTokens, T: Display>(&self, obj: A, msg: T) {
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            // Curb monomorphization from generating too many identical methods.
            .push(syn::Error::new_spanned(obj.into_token_stream(), msg));
    }

    /// Consume this object, producing a formatted error string if there are errors.
    pub(crate) fn check(self) -> Result<(), Vec<syn::Error>> {
        let errors = self.errors.borrow_mut().take().unwrap();
        match errors.len() {
            0 => Ok(()),
            _ => Err(errors),
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !thread::panicking() && self.errors.borrow().is_some() {
            panic!("forgot to check for errors");
        }
    }
}
