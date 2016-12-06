use std::process;
use std::io;
use std::io::Write;

extern crate amqp;
use amqp::{Session, Options, Channel};
use amqp::protocol::basic::BasicProperties;
use amqp::Basic;
use amqp::{Table, TableEntry};

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
    pub headers: Vec<String>,
    pub reader: Box<io::Read>,
}

fn _open(o:Options) -> (Session, Channel) {
    let mut session = Session::new(o)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    let channel = session.open_channel(1)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    (session, channel)
}

pub fn open_send(o:Options, s:Sendable) {
    let (mut session, mut channel) = _open(o);
    let headers = s.headers.iter().fold(Table::new(), |mut h, st| {
        let idx = st.find(':').expect("Header must have a :");
        let (name, value) = st.split_at(idx);
        let key = name.trim();
        let val = (&value[1..]).trim();
        h.insert(String::from(key), TableEntry::LongString(String::from(val)));
        h
    });
    let props = BasicProperties {
        content_type: Some(s.content_type),
        headers: Some(headers),
        ..Default::default()
    };
    let mut buffer = Box::new(vec![]);
    let mut reader = s.reader;
    reader.read_to_end(&mut buffer)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    channel.basic_publish(s.exchange, s.routing_key, false, false, props, *buffer)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    channel.close(200, "Bye")
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    session.close(200, "Good Bye");
}
