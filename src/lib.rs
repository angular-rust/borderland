use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

mod route;
mod router;

pub use self::route::{Handler, Matcher, ReadWrite, Route};
pub use self::router::Router;

pub enum Method {
    OPTIONS,
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    TRACE,
    CONNECT,
    PATCH,
    Extension(String),
}

impl FromStr for Method {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.len() {
            3 => match s.as_ref() {
                "GET" => Ok(Method::GET),
                "PUT" => Ok(Method::PUT),
                _ => Ok(Method::Extension(s.to_string())),
            },
            4 => match s.as_ref() {
                "POST" => Ok(Method::POST),
                "HEAD" => Ok(Method::HEAD),
                _ => Ok(Method::Extension(s.to_string())),
            },
            5 => match s.as_ref() {
                "PATCH" => Ok(Method::PATCH),
                "TRACE" => Ok(Method::TRACE),
                _ => Ok(Method::Extension(s.to_string())),
            },
            6 => match s.as_ref() {
                "DELETE" => Ok(Method::DELETE),
                _ => Ok(Method::Extension(s.to_string())),
            },
            7 => match s.as_ref() {
                "OPTIONS" => Ok(Method::OPTIONS),
                "CONNECT" => Ok(Method::CONNECT),
                _ => Ok(Method::Extension(s.to_string())),
            },
            _ => Ok(Method::Extension(s.to_string())),
        }
    }
}

impl fmt::Debug for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GET")
    }
}
