use super::*;
#[cfg(feature = "std")]
impl core::fmt::Display for ErrorData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(c) = &self.cause {
            c.fmt(f)
        } else {
            let c = self
                .payload
                .iter()
                .find(|p| matches!(p, PayloadEntry::Cause { .. }));
            if let Some(PayloadEntry::Cause { display, .. }) = c {
                f.pad(display)
            } else {
                // Best we can do, I suppose.
                write!(f, "Ockam error [{}] at {}", self.code, self.source_loc)
            }
        }
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for ErrorData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Ockam Error [{}] at {}", self.code, self.source_loc)?;
        writeln!(f, "--- Details ---")?;
        if let Some(c) = &self.cause {
            // TODO: iterate the cause chain, print the whole thing out
            writeln!(f, "- Caused by: {}", c)?;
        }
        for e in self.payload.iter() {
            match e {
                PayloadEntry::Info(k, v) => {
                    writeln!(f, "- info {:?}: {}", k, v)?;
                }
                PayloadEntry::Cause {
                    display,
                    debug,
                    type_name,
                } => {
                    writeln!(f, "- possible root cause `{}`: {}", type_name, display)?;
                    writeln!(f, "  debug output: ")?;
                    for line in debug.lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
            }
        }

        if let Some(local) = self.local.as_ref() {
            for e in local {
                match e {
                    #[cfg(all(feature = "std", feature = "tracing-error"))]
                    LocalPayloadEntry::Spantrace(t) => {
                        writeln!(f, "--- Captured Spantrace ---")?;
                        writeln!(f, "{}", t)?;
                    }
                    #[cfg(all(feature = "std", feature = "backtrace"))]
                    LocalPayloadEntry::Backtrace(t) => {
                        writeln!(f, "--- Captured Backtrace ---")?;
                        // TODO: Finish up filtering out pointless frames -- see
                        // https://gist.github.com/thomcc/0e9f1cc4dd9f6f12943a158eeb848f7e
                        writeln!(f, "{:?}", t)?;
                    }
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
            }
        }
        writeln!(f, "--------------------------")?;
        Ok(())
    }
}

#[cfg(not(feature = "std"))]
impl core::fmt::Display for ErrorData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // TODO
        write!(f, "Ockam Error [{}]", self.code)
    }
}
#[cfg(not(feature = "std"))]
impl core::fmt::Debug for ErrorData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // TODO
        write!(f, "Ockam Error [{}]", self.code)
    }
}
