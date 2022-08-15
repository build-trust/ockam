use core::future::Future;
pub use core::time::Duration;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    pub struct Timeout<F> {
        #[pin]
        duration: Duration,
        #[pin]
        future: F,
    }
}

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout { duration, future }
}

impl<F> Future for Timeout<F>
where
    F: Future,
{
    type Output = Result<F::Output, error::Elapsed>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let timeout = self.project();

        // try polling the future
        if let core::task::Poll::Ready(v) = timeout.future.poll(cx) {
            return core::task::Poll::Ready(Ok(v));
        }

        // TODO check the timer
        /*match timeout.delay.poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(Elapsed::new())),
            Poll::Pending => Poll::Pending,
        }*/
        core::task::Poll::Pending
    }
}

pub struct Sleep;

pub async fn sleep(_duration: Duration) -> Sleep {
    unimplemented!();
}

pub mod error {
    use core::fmt;
    use ockam_core::compat::{error, io};

    #[derive(Debug, PartialEq, Eq)]
    pub struct Elapsed(());

    impl Elapsed {
        #![allow(dead_code)]
        pub(crate) fn new() -> Self {
            Elapsed(())
        }
    }

    impl fmt::Display for Elapsed {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            "deadline has elapsed".fmt(fmt)
        }
    }

    impl error::Error for Elapsed {}

    impl From<Elapsed> for io::Error {
        fn from(_err: Elapsed) -> io::Error {
            io::ErrorKind::TimedOut.into()
        }
    }
}
