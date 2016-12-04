use std::process;
use std::io;
use std::io::Write;

extern crate amqp;
use amqp::{Session, Options, Channel};
use amqp::protocol::basic::BasicProperties;
use amqp::Basic;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
        process::exit(1);
    } }
);

pub struct Sendable {
    pub exchange: String,
    pub routing_key: String,
    pub content_type: String,
    pub reader: Box<io::Read>,
}

fn _open(o:Options) -> (Session, Channel) {
    let mut session = match Session::new(o) {
        Ok(session) => session,
        Err(error)  => exitln!("Failed opening amqp session: {:?}", error),
    };
    let channel = match session.open_channel(1) {
        Ok(channel) => channel,
        Err(error)  => exitln!("Failed opening amqp channel: {:?}", error),
    };
    (session, channel)
}

pub fn open_send(o:Options, s:Sendable) {
    let (mut session, mut channel) = _open(o);
    // exchange: S,
    // routing_key: S,
    // mandatory: bool,
    // immediate: bool,
    // properties: BasicProperties,
    // content: Vec<u8>
    let props = BasicProperties { content_type: Some(s.content_type), ..Default::default() };
    let mut buffer = Vec::new();
    let mut reader = s.reader;
    match reader.read_to_end(&mut buffer) {
        Ok(_)      => (),
        Err(error) => exitln!("Failed to read input: {:?}", error)
    }
    match channel.basic_publish(s.exchange, s.routing_key, false, false, props, buffer) {
        Ok(_)      => (),
        Err(error) => exitln!("Failed to send message: {:?}", error),
    }
    match channel.close(200, "Bye") {
        Ok(_)      => (),
        Err(error) => exitln!("Failed to close channel: {:?}", error),
    };
    session.close(200, "Good Bye");
}
