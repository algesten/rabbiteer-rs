use std::io;

use error::RbtError;
use amqp::{self, Session, Options, Channel};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::Basic;
use amqp::{Table, TableEntry};



pub struct Sendable {
    pub exchange: String,
    pub routing_key: String,
    pub content_type: String,
    pub headers: Vec<String>,
    pub file_name: String,
    pub reader: Box<io::Read>,
}

pub fn open_send(o:Options, s:Sendable) -> Result<(),RbtError> {
    let (mut session, mut channel) = try!(_open(o));
    let mut headers = Table::new();
    for st in s.headers {
        let idx = try!(st.find(':').ok_or("Header must have a :"));
        let (name, value) = st.split_at(idx);
        let key = name.trim();
        let valstr = (&value[1..]).trim();
        let val = narrow(valstr);
        headers.insert(String::from(key), val);
    }
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
    try!(reader.read_to_end(&mut buffer));
    try!(channel.basic_publish(s.exchange, s.routing_key, false, false, props, *buffer));
    try!(channel.close(200, "Bye"));
    session.close(200, "Good Bye");
    Ok(())
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


fn _open(o:Options) -> Result<(Session, Channel),RbtError> {
    errln!("Connecting to amqp://{}:{}@{}:{}/{}",
           o.login, o.password, o.host, o.port, o.vhost);
    let mut session = try!(Session::new(o));
    let channel = try!(session.open_channel(1));
    Ok((session, channel))
}



pub struct Receiver {
    pub exchange:String,
    pub routing_key:String,
    pub callback:Box<FnMut(Deliver, BasicProperties, Vec<u8>) -> Result<(), RbtError> + Send>,
}

impl amqp::Consumer for Receiver {
    fn handle_delivery(&mut self, channel:&mut Channel, deliver:Deliver,
                       headers:BasicProperties, body:Vec<u8>){

        let delivery_tag = deliver.delivery_tag.clone();

        // and deliver to callback
        ((self.callback)(deliver, headers, body)).unwrap_or_else(::error::handle);

        // ack it. not that we're in ack mode...
        channel.basic_ack(delivery_tag, false).unwrap();

    }
}

pub fn open_receive(o:Options, r:Receiver) -> Result<(),RbtError> {

    // open session/channel
    let (_, mut channel) = try!(_open(o));

    // declare an exclusive anonymous queue that auto deletes
    // when the process exits.
    // queue, passive, durable, exclusive, auto_delete, nowait,  arguments
    let queue_declare =
        try!(channel.queue_declare("", false, false, true, true, false, Table::new()));

    // name is auto generated
    let queue_name = queue_declare.queue;

    // bind queue to the exchange, which already must
    // be declared.
    try!(channel.queue_bind(queue_name.clone(), r.exchange.clone(), r.routing_key.clone(),
                            false, Table::new()));

    // why oh why?
    let consumer_tag = String::from("");

    // start consuming the queue.
    try!(channel.basic_consume(r, queue_name, consumer_tag, false,
                               false, false, false, Table::new()));

    // and go!
    channel.start_consuming();


    Ok(())
}
