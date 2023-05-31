use std::{env::VarError, net::AddrParseError, num::ParseIntError};

use lettre::address::AddressError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("generic error: {0}")]
    Generic(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("error parsing json: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("generic email error: {0}")]
    Lettre(#[from] lettre::error::Error),
    #[error("smtp error: {0}")]
    LettreSmtp(#[from] lettre::transport::smtp::Error),
    #[error("environment variable error: {0}")]
    Var(#[from] VarError),
    #[error("address parsing error: {0}")]
    AddrParse(#[from] AddrParseError),
    #[error("email address error: {0}")]
    Address(#[from] AddressError),
    #[error("int parsing error: {0}")]
    ParseInt(#[from] ParseIntError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{self}"))
    }
}
