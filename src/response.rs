use serde::Serialize;

use crate::error::Error;

#[derive(Debug, Serialize)]
pub struct Response<T> {
    status: Status,
    message: LocalResult<T>,
}

impl<T> Response<T> {
    pub fn ok(val: T) -> Self {
        Self {
            status: Status::OK,
            message: LocalResult::Ok(val),
        }
    }

    pub fn err(err: Error) -> Self {
        Self {
            status: Status::KO,
            message: LocalResult::Err(err),
        }
    }
}

#[derive(Debug, Default)]
enum Status {
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

#[derive(Debug)]
enum LocalResult<T> {
    Ok(T),
    Err(Error),
}

impl<T> Serialize for LocalResult<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LocalResult::Ok(val) => val.serialize(serializer),
            LocalResult::Err(err) => err.serialize(serializer),
        }
    }
}
