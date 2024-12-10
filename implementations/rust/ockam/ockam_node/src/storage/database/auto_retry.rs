/// This macro wraps a function with retry calls to keep calling the function until
/// the error "database is locked" is not raised anymore.
#[macro_export]
macro_rules! retry {
    ($async_function:expr) => {{
        let mut retries = 0;
        loop {
            match $async_function.await {
                Ok(result) => break Ok(result),
                Err(err) => {
                    if err.to_string().contains("database is locked") && retries < 100 {
                        ockam_node::tokio::time::sleep(
                            ockam_node::tokio::time::Duration::from_millis(10),
                        )
                        .await;
                    } else {
                        break Err(err);
                    }
                    retries += 1;
                }
            }
        }
    }};
}

/// Wrapper for an auto-retried struct
#[derive(Clone)]
pub struct AutoRetry<T: Sized + Send + Sync + 'static> {
    /// Internal implementation of the trait
    pub wrapped: T,
}

impl<T: Send + Sync + 'static> AutoRetry<T> {
    /// Wrap a trait so that it is auto-retried
    pub fn new(wrapped_trait: T) -> AutoRetry<T> {
        Self {
            wrapped: wrapped_trait,
        }
    }
}
