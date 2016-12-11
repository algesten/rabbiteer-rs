use std::fmt;
use std::io;
use std::convert::From;
use amqp::AMQPError;
use std::string::FromUtf8Error;
use rustc_serialize::json;
use clap;

#[macro_export]
macro_rules! errln(
    ($($arg:tt)*) => {{
        use std::io::Write;
        writeln!(&mut ::std::io::stderr(), $($arg)*).expect("failed printing to stderr");
    }}
);

#[macro_export]
macro_rules! rbterr(
    ($($arg:tt)*) => {{
        return Err(::error::RbtError::Message(format!($($arg)*)));
    }}
);


pub enum RbtError {
    Message(String),  // Plain error message
    AMQP(AMQPError),
    IO(io::Error),
    UTF8(FromUtf8Error),
    JSON(json::ParserError),
    Clap(clap::Error),
}


impl fmt::Display for RbtError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            RbtError::Message(ref s) => write!(f, "Error: {}", s),
            RbtError::AMQP(ref e)    => write!(f, "{}", e),
            RbtError::IO(ref e)      => write!(f, "{}", e),
            RbtError::UTF8(ref e)    => write!(f, "{}", e),
            RbtError::JSON(ref e)    => write!(f, "{}", e),
            RbtError::Clap(ref e)    => write!(f, "{}", e),
        }
    }
}


pub fn handle(e:RbtError) {
    errln!("{}", e);
    ::std::process::exit(1);
}


macro_rules! from(
    ($t:ty, $p:tt) => {
        impl From<$t> for RbtError {
            fn from(err:$t) -> RbtError {
                RbtError::$p(err)
            }
        }
    }
 );

from!(AMQPError, AMQP);
from!(io::Error, IO);
from!(FromUtf8Error, UTF8);
from!(json::ParserError, JSON);
from!(clap::Error, Clap);
from!(String, Message);

impl From<&'static str> for RbtError {
    fn from(s:&str) -> RbtError {
        RbtError::Message(String::from(s))
    }
}
