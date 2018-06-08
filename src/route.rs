use std::io::{Read, Write};

pub trait ReadWrite: Read + Write {}
impl<T> ReadWrite for T
where
    T: Read + Write,
{
}

pub type Handler = fn(ReadWrite);

pub trait Matcher {
    // fn summarize(&self) -> String;
}

#[allow(dead_code)]
pub struct Route {
    matcher: Box<Matcher>,
    handler: Handler,
}

impl Route {
    pub fn new(matcher: Box<Matcher>, handler: Handler) -> Route {
        Route {
            matcher: matcher,
            handler: handler,
        }
    }
}
