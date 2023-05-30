use std::{env::VarError, net::AddrParseError, num::ParseIntError};

#[derive(Debug)]
pub enum Error {
    Generic(String),
    Io(std::io::Error),
    Serde(serde_json::Error),
    Var(VarError),
    AddrParse(AddrParseError),
    ParseInt(ParseIntError),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<VarError> for Error {
    fn from(value: std::env::VarError) -> Self {
        Self::Var(value)
    }
}

impl From<AddrParseError> for Error {
    fn from(value: AddrParseError) -> Self {
        Self::AddrParse(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}
