use std::io;
use std::convert::From;
use amqp::AMQPError;
use std::string::FromUtf8Error;
use rustc_serialize::json;
use clap;

#[derive(Debug)]
pub enum RbtError {
    Message(String),  // Plain error message
    AMQP(AMQPError),
    IO(io::Error),
    UTF8(FromUtf8Error),
    JSONParse(json::ParserError),
    Clap(clap::Error),
}


impl From<AMQPError> for RbtError {
    fn from(err:AMQPError) -> RbtError {
        RbtError::AMQP(err)
    }
}

impl From<io::Error> for RbtError {
    fn from(err:io::Error) -> RbtError {
        RbtError::IO(err)
    }
}

impl From<FromUtf8Error> for RbtError {
    fn from(err:FromUtf8Error) -> RbtError {
        RbtError::UTF8(err)
    }
}

impl From<json::ParserError> for RbtError {
    fn from(err:json::ParserError) -> RbtError {
        RbtError::JSONParse(err)
    }
}

impl From<clap::Error> for RbtError {
    fn from(err:clap::Error) -> RbtError {
        RbtError::Clap(err)
    }
}

impl From<&'static str> for RbtError {
    fn from(s:&str) -> RbtError {
        RbtError::Message(String::from(s))
    }
}

impl From<String> for RbtError {
    fn from(s:String) -> RbtError {
        RbtError::Message(s)
    }
}
