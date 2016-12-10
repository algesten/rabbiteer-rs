use std::process;
use std::io::{self, Write};

use amqp::{self, Session, Options, Channel};
use amqp::protocol::basic::{Deliver, BasicProperties};
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
    pub file_name: String,
    pub reader: Box<io::Read>,
}

pub fn open_send(o:Options, s:Sendable) {
    let (mut session, mut channel) = _open(o);
    let mut headers = s.headers.iter().fold(Table::new(), |mut h, st| {
        let idx = st.find(':').expect("Header must have a :");
        let (name, value) = st.split_at(idx);
        let key = name.trim();
        let valstr = (&value[1..]).trim();
        let val = narrow(valstr);
        h.insert(String::from(key), val);
        h
    });
    if s.file_name != "-" && !headers.contains_key("fileName") {
        headers.insert("fileName".to_owned(), TableEntry::LongString(String::from(s.file_name)));
    }
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


// narrow the string to a TableEntry type by trying to parse to known
// JSON types: bool, double and fall back on string.
fn narrow(str:&str) -> TableEntry {
    let boolv = str.parse::<bool>();
    if !boolv.is_err() {
        TableEntry::Bool(boolv.unwrap())
    } else {
        let doublev = str.parse::<f64>();
        if !doublev.is_err() {
            TableEntry::Double(doublev.unwrap())
        } else {
            TableEntry::LongString(str.to_owned())
        }
    }
}


fn _open(o:Options) -> (Session, Channel) {
    let mut session = Session::new(o)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    let channel = session.open_channel(1)
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    (session, channel)
}



pub type ReceiverCallback = fn(deliver:Deliver,
                               props:BasicProperties,
                               body:Vec<u8>);

pub struct Receiver {
    pub exchange:String,
    pub routing_key:String,
    pub callback:Box<FnMut(Deliver, BasicProperties, Vec<u8>) + Send>,
}

impl amqp::Consumer for Receiver {
    fn handle_delivery(&mut self, channel:&mut Channel, deliver:Deliver,
                       headers:BasicProperties, body:Vec<u8>){

        let delivery_tag = deliver.delivery_tag.clone();

        // and deliver to callback
        (self.callback)(deliver, headers, body);

        // ack it. not that we're in ack mode...
        channel.basic_ack(delivery_tag, false).unwrap();

    }
}

pub fn open_receive(o:Options, r:Receiver) {

    // open session/channel
    let (_, mut channel) = _open(o);

    // declare an exclusive anonymous queue that auto deletes
    // when the process exits.
    // queue, passive, durable, exclusive, auto_delete, nowait,  arguments
    let queue_declare =
        channel.queue_declare("", false, false, true,
                              true, false, Table::new())
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));

    // name is auto generated
    let queue_name = queue_declare.queue;

    // bind queue to the exchange, which already must
    // be declared.
    channel.queue_bind(queue_name.clone(), r.exchange.clone(), r.routing_key.clone(),
                       false, Table::new())
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));

    // why oh why?
    let consumer_tag = String::from("");

    // start consuming the queue.
    channel.basic_consume(r, queue_name, consumer_tag, false,
                          false, false, false, Table::new())
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));

    // and go!
    channel.start_consuming();

}
