use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct Error {
    inner: Inner,
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        if let Inner::NotFound = self.inner {
            true
        } else {
            false
        }
    }
    pub fn is_forbidden(&self) -> bool {
        if let Inner::Forbidden = self.inner {
            true
        } else {
            false
        }
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        match e.status() {
            Some(status) if status.as_u16() == 404 => Inner::NotFound.into(),
            Some(status) if status.as_u16() == 403 => Inner::Forbidden.into(),
            _ => Inner::from(e).into(),
        }
    }
}
impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self { inner: e.into() }
    }
}

impl From<Inner> for Error {
    fn from(inner: Inner) -> Self {
        Self { inner }
    }
}

#[derive(thiserror::Error, Debug)]
enum Inner {
    #[error("Requested resource not found")]
    NotFound,
    #[error("Access to requested resource is forbidden")]
    Forbidden,
    #[error("An error occurred: {}", .0)]
    HttpClient(#[from] reqwest::Error),
    #[error("An error occurred: {}", .0)]
    Other(#[from] anyhow::Error),
}
