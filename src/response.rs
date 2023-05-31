use serde::Serialize;

use crate::error::Error;

#[derive(Debug, Serialize)]
pub struct Response<T> {
    status: Status,
    message: Result<T, Error>,
}

impl<T> Response<T> {
    pub fn ok(val: T) -> Self {
        Self {
            status: Status::OK,
            message: Ok(val),
        }
    }

    pub fn err(err: Error) -> Self {
        Self {
            status: Status::KO,
            message: Err(err),
        }
    }
}

#[derive(Debug, Default)]
pub enum Status {
    #[default]
    OK,
    KO,
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Status::OK => serializer.serialize_str("ok"),
            Status::KO => serializer.serialize_str("ko"),
        }
    }
}
