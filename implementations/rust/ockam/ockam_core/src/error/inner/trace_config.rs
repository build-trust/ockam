//! Note: This module is private, and only enabled if both `std` and
//! `error-traces` are on.
use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(feature = "std")]
pub(super) static BACKTRACE_ENABLED: TraceConfig = TraceConfig::new(
    &["OCKAM_BACKTRACE", "RUST_LIB_BACKTRACE", "RUST_BACKTRACE"],
    false,
);

#[cfg(feature = "std")]
pub(super) static SPANTRACE_ENABLED: TraceConfig = TraceConfig::new(
    &[
        "OCKAM_SPANTRACE",
        "OCKAM_BACKTRACE",
        "RUST_LIB_BACKTRACE",
        "RUST_BACKTRACE",
    ],
    true,
);

pub(super) struct TraceConfig {
    state: AtomicU8,
    vars: &'static [&'static str],
    default: bool,
}

impl TraceConfig {
    pub const fn new(vars: &'static [&'static str], default: bool) -> Self {
        Self {
            state: AtomicU8::new(encode_opt_bool(None)),
            vars,
            default,
        }
    }
    pub(super) fn get(&self) -> bool {
        are_traces_enabled(&self.state, || {
            check_env_vars(self.vars).unwrap_or(self.default)
        })
    }
}

fn check_env_vars(vars: &[&str]) -> Option<bool> {
    vars.iter()
        .copied()
        .find_map(|name| std::env::var_os(name))
        .map(|val| val != "0")
}

// This is generic for testing — call `are_backtraces_enabled()` instead.
fn are_traces_enabled(state: &AtomicU8, read_env: impl FnOnce() -> bool) -> bool {
    // Relaxed is fine here, since we are only interested in the effects on a
    // single memory location.
    match decode_opt_bool(state.load(Ordering::Relaxed)) {
        Some(b) => b,
        None => {
            let enabled = read_env();
            state.store(encode_opt_bool(Some(enabled)), Ordering::Relaxed);
            enabled
        }
    }
}

// Conceptually stores an `Option<bool>`, which stores the cached result of
// `env_backtrace_enabled()`. It uses 0 for None, and 1 plus the bool value
// for anything else (false is 1, true is 2).
fn decode_opt_bool(n: u8) -> Option<bool> {
    match n {
        0 => None,
        n => Some((n - 1) != 0),
    }
}
const fn encode_opt_bool(u: Option<bool>) -> u8 {
    match u {
        None => 0,
        Some(b) => b as u8 + 1,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ffi::{OsStr, OsString};
    #[test]
    fn test_bool_enc() {
        assert_eq!(decode_opt_bool(encode_opt_bool(None)), None);
        assert_eq!(decode_opt_bool(encode_opt_bool(Some(false))), Some(false));
        assert_eq!(decode_opt_bool(encode_opt_bool(Some(true))), Some(true));
        assert_eq!(encode_opt_bool(decode_opt_bool(0)), 0);
        assert_eq!(encode_opt_bool(decode_opt_bool(1)), 1);
        assert_eq!(encode_opt_bool(decode_opt_bool(2)), 2);
    }

    // RAII type with a `Drop` which restores an env var to the value it had
    // previously.
    //
    // (To fix this, we could probably take advantage of the fact that
    // `check_env_vars` and such accepts a list of arbitrary varnames, and use a
    // set of random, or otherwise externally meaningless, vars. I don't *think*
    // that would cause any problems...)
    struct AutoVar {
        name: String,
        val: Option<OsString>,
    }
    impl AutoVar {
        fn new(name: impl Into<String>) -> Self {
            let name = name.into();
            let val = std::env::var_os(&name);
            Self { name, val }
        }
    }
    impl Drop for AutoVar {
        fn drop(&mut self) {
            force_var(&self.name, self.val.as_ref().map(|v| &**v));
        }
    }
    // like env::set_var, but takes an option and unsets the var on `None`
    fn force_var<V: AsRef<OsStr>>(name: &str, value: Option<V>) {
        match value {
            None => std::env::remove_var(name),
            Some(v) => std::env::set_var(name, v),
        }
    }

    #[test]
    fn test_env() {
        #[track_caller]
        fn testcase(
            ockam_bt: Option<&'static str>,
            rust_lib_bt: Option<&'static str>,
            rust_bt: Option<&'static str>,
            want: bool,
        ) {
            // We avoid the real var names here, and use
            // `__FAKE_VAR_FOR_TESTS`-suffixed equivalents, to avoid a situation
            // where an unrelated test running concurrently to this one fails
            // and doesn't have any backtrace info because it happened during a
            // period of time where we disabled `RUST_BACKTRACE` or
            // `RUST_LIB_BACKTRACE` or whatever. This would only be relevant for
            // unit tests inside `ockam_core`, but would be annoying
            // nonetheless.
            //
            // This also saves us from having to explicitly `drop(guards)`
            // before the assert at the end of this function -- otherwise we'd
            // hit this problem here.
            let _guards = [
                AutoVar::new("OCKAM_BACKTRACE__FAKE_VAR_FOR_TESTS"),
                AutoVar::new("RUST_LIB_BACKTRACE__FAKE_VAR_FOR_TESTS"),
                AutoVar::new("RUST_BACKTRACE__FAKE_VAR_FOR_TESTS"),
            ];
            force_var("OCKAM_BACKTRACE__FAKE_VAR_FOR_TESTS", ockam_bt);
            force_var("RUST_LIB_BACKTRACE__FAKE_VAR_FOR_TESTS", rust_lib_bt);
            force_var("RUST_BACKTRACE__FAKE_VAR_FOR_TESTS", rust_bt);
            let got = check_env_vars(&[
                "OCKAM_BACKTRACE__FAKE_VAR_FOR_TESTS",
                "RUST_LIB_BACKTRACE__FAKE_VAR_FOR_TESTS",
                "RUST_BACKTRACE__FAKE_VAR_FOR_TESTS",
            ]);
            assert_eq!(
                got.unwrap_or(false),
                want,
                "ockam: {:?}, rust_lib: {:?}, rust: {:?}, interpreted as {:?}",
                ockam_bt,
                rust_lib_bt,
                rust_bt,
                got,
            );
        }
        testcase(None, None, None, false);
        for filler in [None, Some(""), Some("1"), Some("0")] {
            testcase(Some(""), filler, filler, true);
            testcase(None, Some(""), filler, true);
            testcase(None, None, Some(""), true);
            testcase(Some("0"), filler, filler, false);
            testcase(None, Some("0"), filler, false);
            testcase(None, None, Some("0"), false);
        }
    }
}
