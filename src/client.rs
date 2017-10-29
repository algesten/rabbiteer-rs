use std::io;
use error::RbtError;
use amqp::{self, Session, Options, Channel};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::Basic;
use amqp::{Table, TableEntry};

use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use std::error::Error;

pub struct Sendable {
    pub exchange: String,
    pub routing_key: String,
    pub content_type: String,
    pub headers: Vec<String>,
    pub file_name: String,
    pub reader: Box<io::Read>,
    pub priority: u8,
    pub rpctimeout: u64
}

pub type ReceiveCb = FnMut(&mut Channel, Deliver, BasicProperties, Vec<u8>) -> Result<(), RbtError> + Send;

pub struct Receiver {
    pub exchange:String,
    pub routing_key: Option<String>,
    pub auto_ack: bool,
    pub callback:Box<ReceiveCb>,
}


pub fn open_send(o:Options, s:Sendable, r:Option<Receiver>) -> Result<(),RbtError> {

    // open the channel
    let (mut session, mut channel) = _open(o)?;

    // table of headers, parsed from input
    let mut headers = Table::new();
    for st in s.headers {
        let idx = st.find(':').ok_or("Header must have a :")?;
        let (name, value) = st.split_at(idx);
        let key = name.trim();
        let valstr = (&value[1..]).trim();
        let val = narrow(valstr);
        headers.insert(String::from(key), val);
    }

    // put filename in headers if we read from file
    if s.file_name != "-" && !headers.contains_key("fileName") {
        headers.insert("fileName".to_owned(), TableEntry::LongString(String::from(s.file_name)));
    }

    // send properties
    let mut props = BasicProperties {
        content_type: Some(s.content_type),
        headers: Some(headers),
        priority: Some(s.priority),
        ..Default::default()
    };

    // if we are doing rpc, there is a receiver in this optional
    let isrpc = match r {
        Some(receiver) => {
            // open a receiver and get the queue name
            let queue_name = do_open_receive(&mut channel, None, false, receiver)?;

            // put queue name as our reply to
            props.reply_to = Some(queue_name);

            // and a fixed correltionId
            props.correlation_id = Some("rabbiteer here".to_owned());

            true
        },
        None => false
    };

    // read input input buffer
    let mut buffer = Box::new(vec![]);
    let mut reader = s.reader;
    reader.read_to_end(&mut buffer)?;

    // publish it
    channel.basic_publish(s.exchange, s.routing_key, false, false, props, *buffer)?;

    if isrpc {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new().name("consumer_thread".to_string()).spawn(move || {
            channel.start_consuming();
            tx.send(channel).unwrap();
        }).unwrap();


        let timeout = s.rpctimeout;
        if timeout == 0 {
            // Block forever until recieve
            rx.recv().unwrap();
        } else {
            let res = rx.recv_timeout(Duration::from_millis(timeout));

            match res {
                Ok(mut ch) => {
                    ch.close(200, "Bye")?;
                }                  
                Err(err) => {
                    if err.description() == "timed out waiting on channel".to_string() {
                        println!("Error timeout");
                    }
                }
            }
        }
        session.close(200, "Good Bye");
    } else {
        // and unwind if not rpc
        channel.close(200, "Bye")?;
        session.close(200, "Good Bye");
    }

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
//    errln!("Connecting to amqp://{}:{}@{}:{}/{}",
//           o.login, o.password, o.host, o.port, o.vhost);
    let mut session = Session::new(o)?;
    let channel = session.open_channel(1)?;
    Ok((session, channel))
}

impl amqp::Consumer for Receiver {
    fn handle_delivery(&mut self, channel:&mut Channel, deliver:Deliver,
                       headers:BasicProperties, body:Vec<u8>){

        let delivery_tag = deliver.delivery_tag.clone();

        if self.auto_ack {
            // ack it. make it go away.
            channel.basic_ack(delivery_tag, false).unwrap();
        }

        // and deliver to callback
        ((self.callback)(channel, deliver, headers, body)).unwrap_or_else(::error::handle);

    }
}

pub fn open_receive(o:Options, q:Option<String>, force_declare: bool, r:Receiver) -> Result<(),RbtError> {

    // open session/channel
    let (_, mut channel) = _open(o)?;

    // and pass it to internal open_receive
    do_open_receive(&mut channel, q, force_declare, r)?;

    // and go!
    channel.start_consuming();

    Ok(())
}


fn do_open_receive(channel:&mut Channel, q:Option<String>, force_declare: bool, r:Receiver) -> Result<String,RbtError> {

    let mut auto_delete = false;
    let mut bind_routing_key = r.routing_key.clone();
    
    let queue_name = match q {
        Some(q) => {
            // Force the declaration of this queue
            if force_declare {
                // queue, passive, durable, exclusive, auto_delete, nowait, arguments
                let queue_declare =  channel.queue_declare(q, false, false, auto_delete, auto_delete, false, Table::new())?;

                // name is auto generated
                queue_declare.queue
            }else{
                q
            }
        }
        None => {
            auto_delete = true; // Unnamed queues are ephemeral

            if let None = bind_routing_key {
                bind_routing_key = Some("#".to_owned()); // Default the routing key
            }

            // queue, passive, durable, exclusive, auto_delete, nowait, arguments
            let queue_declare =
                channel.queue_declare(
                                    q.clone().unwrap_or("".to_owned()) ,
                                    false, false,
                                    auto_delete, auto_delete, false, Table::new())?;

            // name is auto generated
            queue_declare.queue

        }
    };

    // Only bind if we have a routing key - May be an existing queue
    if let Some(routing_key) = bind_routing_key {
        // bind queue to the exchange, which already must be declared.
        
        if r.exchange != "" {
            channel.queue_bind(queue_name.clone(), r.exchange.clone(), routing_key.clone(),
                            false, Table::new())?;
        }
    }

    // why oh why?
    let consumer_tag = "".to_string();

    // start consuming the queue.
    // callback, queue, consumer_tag, no_local, no_ack, exclusive, nowait, arguments
    channel.basic_consume(r, queue_name.clone(), consumer_tag, false,
                          false, false, false, Table::new())?;

    Ok(queue_name)
}
