#![allow(dead_code)]
/// Alias for the numeric type that holds system exit codes.
pub type ExitCode = i32;

/// Successful exit
pub const OK: ExitCode = 0;

/// The command was used incorrectly, e.g., with the
/// wrong number of arguments, a bad flag, a bad syntax
/// in a parameter, etc.
pub const USAGE: ExitCode = 64;

/// The input data was incorrect in some way.  This
/// should only be used for user's data and not system
/// files.
pub const DATAERR: ExitCode = 65;

/// An input file (not a system file) did not exist or
/// was not readable.  This could also include errors
/// like "No message" to a mailer (if it cared to
/// catch it).
pub const NOINPUT: ExitCode = 66;

/// The user specified did not exist.  This might be
/// used for mail addresses or remote logins.
pub const NOUSER: ExitCode = 67;

/// The host specified did not exist.  This is used in
/// mail addresses or network requests.
pub const NOHOST: ExitCode = 68;

/// A service is unavailable.  This can occur if a
/// support program or file does not exist. This can also
/// be used as a catchall message when something you
/// wanted to do doesn't work, but you don't know why.
pub const UNAVAILABLE: ExitCode = 69;

/// An internal software error has been detected.  This
/// should be limited to non-operating system related
/// errors as possible.
pub const SOFTWARE: ExitCode = 70;

/// An operating system error has been detected.  This
/// is intended to be used for such things as "cannot
/// fork", "cannot create pipe", or the like.  It
/// includes things like getuid returning a user that
/// does not exist in the passwd file.
pub const OSERR: ExitCode = 71;

/// Some system file (e.g., /etc/passwd, /var/run/utmp,
/// etc.) does not exist, cannot be opened, or has some
/// sort of error (e.g., syntax error).
pub const OSFILE: ExitCode = 72;

/// A (user specified) output file cannot be created.
pub const CANTCREAT: ExitCode = 73;

/// An error occurred while doing I/O on some file.
pub const IOERR: ExitCode = 74;

/// Temporary failure, indicating something that is not
/// really an error.  In sendmail, this means that a
/// mailer (e.g.) could not create a connection, and
/// the request should be reattempted later.
pub const TEMPFAIL: ExitCode = 75;

/// The remote system returned something that was
/// "not possible" during a protocol exchange.
pub const PROTOCOL: ExitCode = 76;

/// You did not have sufficient permission to perform
/// the operation.  This is not intended for file system
/// problems, which should use `NOINPUT` or `CANTCREAT`,
/// but rather for higher level permissions.
pub const NOPERM: ExitCode = 77;

/// Something was found in an unconfigured or misconfigured state.
pub const CONFIG: ExitCode = 78;
