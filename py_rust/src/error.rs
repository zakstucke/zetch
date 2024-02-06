use error_stack::Context;
use strum::Display;

/// All errors propagated through the system, everything other than InternalError should be a user problem.
#[derive(Debug, Display)]
pub enum Zerr {
    /// Something that's gone wrong with reading and parsing the config file.
    ConfigInvalid,
    /// When config already exists at the target location and the init command is attempted.
    ConfigExistsError,
    /// When there's something wrong with the supplied render root directory.
    RootError,
    /// When a user configured command returns a non-zero exit code.
    UserCommandError,
    /// When a user variable cannot be coerced using the specified type.
    CoercionError,
    /// When user context cannot be loaded correctly.
    ContextLoadError,
    /// Errors with user supplied python function.
    CustomPyFunctionError,
    /// An error that occurred whilst rendering templates, should be a problem with the user supplied templates, not internal.
    RenderTemplateError,
    /// When a variable requested using subcommand "var" doesn't exist.
    ReadVarMissing,
    /// When the file subcommand is called incorrectly.
    FileCmdUsageError,
    /// When a user specified file cannot be found.
    FileNotFound,
    /// When a user specified file does not match expected syntax.
    FileSyntaxError,
    /// When a user specified path in a file doesn't match the file's structure.
    FilePathError,
    /// When a user puts a value that cannot be used in the current context.
    InvalidPutValue,
    /// When a user task calls zetch commands that call tasks, recursion problem.
    TaskRecursionError,
    /// An unexpected internal error with zetch.
    #[strum(
        serialize = "InternalError: this shouldn't occur, open an issue at https://github.com/zakstucke/zetch/issues"
    )]
    InternalError,
}

impl Context for Zerr {}

/// A macro for building `Report<Zerr>` objects with string context easily.
///
/// E.g. `zerr!(Zerr::ReadConfigError, "Failed to read config file: {}", e)`
#[macro_export]
macro_rules! zerr {
    ($zerr_varient:expr, $str:expr) => {{
        use error_stack::Report;
        use $crate::error::Zerr;

        Report::new($zerr_varient).attach_printable($str)
    }};

    ($zerr_varient:expr, $str:expr, $($arg:expr),*) => {{
        use error_stack::Report;
        use $crate::error::Zerr;

        Report::new($zerr_varient).attach_printable(format!($str, $($arg),*))
    }};
}

/// A macro for building `Report<Zerr>` objects with string context easily.
///
/// E.g. `zerr!(Zerr::ReadConfigError, "Failed to read config file: {}", e)`
#[macro_export]
macro_rules! zerr_int {
    () => {{
        use error_stack::Report;
        use $crate::error::Zerr;

        Report::new(Zerr::InternalError)
    }};

    ($str:expr) => {{
        use error_stack::Report;
        use $crate::error::Zerr;

        Report::new(Zerr::InternalError).attach_printable($str)
    }};

    ($str:expr, $($arg:expr),*) => {{
        use error_stack::Report;
        use $crate::error::Zerr;

        Report::new(Zerr::InternalError).attach_printable(format!($str, $($arg),*))
    }};
}
