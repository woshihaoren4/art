use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Unknown(String),
    EndCallbackError(anyhow::Error),
    ServiceNotFound(String),
    NodeEntityNotFound(String),
    NextNodeNull,
    AnyhowError(anyhow::Error),
}

impl<T> Into<anyhow::Result<T>> for Error {
    fn into(self) -> anyhow::Result<T> {
        Err(anyhow::Error::from(self))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unknown(info) => {
                write!(f, "EndCallbackError:{:?}", info)
            }
            Error::EndCallbackError(err) => {
                write!(f, "EndCallbackError:{:?}", err)
            }
            Error::ServiceNotFound(name) => {
                write!(f, "Service[{}] not found", name)
            }
            Error::NodeEntityNotFound(name) => {
                write!(f, "Node Service Entity [{}] not found", name)
            }
            Error::NextNodeNull => {
                write!(
                    f,
                    ">NextNodeNull< next node is null, service node can not call next function."
                )
            }
            Error::AnyhowError(e) => {
                write!(f, "{:?}", e)
            }
        }
    }
}

impl std::error::Error for Error {}
