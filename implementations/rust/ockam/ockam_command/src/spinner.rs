use indicatif::{ProgressBar, ProgressStyle};
use std::borrow::Cow;

pub struct Spinner {
    inner: ProgressBar,
}

impl Default for Spinner {
    fn default() -> Self {
        let inner = ProgressBar::new_spinner();
        inner.enable_steady_tick(120);
        inner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&[
                    "▰▱▱▱▱▱▱",
                    "▰▰▱▱▱▱▱",
                    "▰▰▰▱▱▱▱",
                    "▰▰▰▰▱▱▱",
                    "▰▰▰▰▰▱▱",
                    "▰▰▰▰▰▰▱",
                    "▰▰▰▰▰▰▰",
                    "▰▱▱▱▱▱▱",
                ])
                .template("{spinner:.blue} {msg}"),
        );
        Self { inner }
    }
}

impl Spinner {
    pub fn stop(&self, msg: impl Into<Cow<'static, str>>) {
        self.inner.finish_with_message(msg);
    }
}
