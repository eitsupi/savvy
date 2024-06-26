use savvy_ffi::SEXP;

#[derive(Debug)]
pub enum Error {
    UnexpectedType(String),
    NotScalar,
    Aborted(SEXP),
    InvalidPointer,
    InvalidRCode(String),
    GeneralError(String),
}

impl Error {
    pub fn new(msg: &str) -> Self {
        Self::GeneralError(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedType(type_name) => write!(f, "Unexpected type: {type_name}"),
            Error::NotScalar => write!(f, "Must be length 1 of non-missing value"),
            Error::Aborted(_) => write!(f, "Aborted due to some error"),
            Error::InvalidPointer => {
                write!(f, "This external pointer is already consumed or deleted")
            }
            Error::InvalidRCode(code) => write!(f, "Failed to parse R code: {code}"),
            Error::GeneralError(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(e: Box<dyn std::error::Error>) -> Error {
        Error::new(&e.to_string())
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Error {
        Error::new(msg)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Error {
        Error::new(&msg)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(value: std::convert::Infallible) -> Self {
        Error::new(&value.to_string())
    }
}
