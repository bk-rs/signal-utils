use core::fmt;

use crate::register::RegisterError;

//
pub mod builder;
mod std_;

pub use builder::Builder;

//
#[derive(Debug)]
pub struct Handler {
    builder: Builder,
}

impl Handler {
    pub fn builder() -> Builder {
        Builder::new()
    }

    pub(crate) fn from_builder(builder: Builder) -> Self {
        Self { builder }
    }
}

//
#[derive(Debug)]
pub enum HandleError {
    AsyncRequired,
    RegisterFailed(RegisterError),
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for HandleError {}
