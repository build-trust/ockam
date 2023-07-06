use crate::enroll;

pub trait Backend {
    fn enroll_user(&self) -> miette::Result<()>;
    fn reset(&self) -> miette::Result<()>;
}

pub struct DefaultBackend;

impl Backend for DefaultBackend {
    fn enroll_user(&self) -> miette::Result<()> {
        enroll::enroll_user::enroll_user()
    }

    fn reset(&self) -> miette::Result<()> {
        enroll::reset::reset()
    }
}
