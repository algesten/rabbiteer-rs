use rustc_serialize::json::{self, Json, Object};
use rustc_serialize::base64::{self, ToBase64};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::TableEntry;
use std::io::Write;
use std::process;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
        process::exit(1);
    } }
);

#[derive(RustcEncodable)]
struct MsgDeliver {
    consumer_tag: String,
    delivery_tag: u64,
    redelivered: bool,
    exchange: String,
    routing_key: String,
}

#[derive(RustcEncodable)]
struct MsgProps {
    content_type: String,
    headers: Object,
}

#[derive(RustcEncodable)]
struct Msg {
    deliver: MsgDeliver,
    props: MsgProps,
    data: Json,
}

pub fn build_output(info:bool, deliver:&Deliver, props:&BasicProperties, body:Vec<u8>) -> Vec<u8> {
    if info {

        // delivery info
        let mdel = MsgDeliver {
            consumer_tag:deliver.consumer_tag.clone(),
            delivery_tag:deliver.delivery_tag.clone(),
            redelivered:deliver.redelivered.clone(),
            exchange:deliver.exchange.clone(),
            routing_key:deliver.routing_key.clone(),
        };

        let content_type = props.content_type.clone().unwrap_or(String::from(""));

        // properties
        let mut mprops = MsgProps {
            content_type:content_type.clone(),
            headers: Object::new(),
        };

        if let Some(table) = props.headers.clone() {
            for (skey, entry) in table {
                let key = skey.to_owned();
                match entry {
                    TableEntry::Bool(v) => {
                        mprops.headers.insert(key, Json::Boolean(v));
                    },
                    TableEntry::ShortShortInt(v) => {
                        mprops.headers.insert(key, Json::I64(v as i64));
                    },
                    TableEntry::ShortShortUint(v) => {
                        mprops.headers.insert(key, Json::U64(v as u64));
                    },
                    TableEntry::ShortInt(v) => {
                        mprops.headers.insert(key, Json::I64(v as i64));
                    },
                    TableEntry::ShortUint(v) => {
                        mprops.headers.insert(key, Json::U64(v as u64));
                    },
                    TableEntry::LongInt(v) => {
                        mprops.headers.insert(key, Json::I64(v as i64));
                    },
                    TableEntry::LongUint(v) => {
                        mprops.headers.insert(key, Json::U64(v as u64));
                    },
                    TableEntry::LongLongInt(v) => {
                        mprops.headers.insert(key, Json::I64(v));
                    },
                    TableEntry::LongLongUint(v) => {
                        mprops.headers.insert(key, Json::U64(v));
                    },
                    TableEntry::Float(v) => {
                        mprops.headers.insert(key, Json::F64(v as f64));
                    },
                    TableEntry::Double(v) => {
                        mprops.headers.insert(key, Json::F64(v));
                    },
                    TableEntry::LongString(v) => {
                        mprops.headers.insert(key, Json::String(v));
                    },
                    TableEntry::Void => {
                        mprops.headers.insert(key, Json::Null);
                    },
                    _ => (),
                    // ShortString(String),
                    // TableEntry::FieldTable(Table) =>
                    // TableEntry::Timestamp(u64) =>
                    // TableEntry::FieldArray(Vec<TableEntry>) =>
                    // TableEntry::DecimalValue(u8, u32) => mprops.headers.insert(key, v),
                };
            }
        }

        // the body
        let data = figure_out_body(content_type, body);

        // and put it together
        let msg = Msg {
            deliver:mdel,
            props:mprops,
            data:data,
        };

        // encode
        let js = json::as_pretty_json(&msg);

        // convert to bytes
        js.to_string().as_bytes().to_owned()

    } else {

        let content_type = props.content_type.clone().unwrap_or(String::from(""));

        match content_type.as_ref() {
            "application/json" => {
                // interpret body
                let body = figure_out_body(content_type, body);

                // encode
                let js = json::as_pretty_json(&body);

                // convert to bytes
                js.to_string().as_bytes().to_owned()
            },
            _ => body
        }

    }
}


fn figure_out_body(content_type:String, body:Vec<u8>) -> Json {

    // interpret body as a binary and base64 encode
    fn as_base64(body:Vec<u8>) -> Json {
        Json::String(body.to_base64(base64::STANDARD))
    }

    fn read_str(body:Vec<u8>) -> String {
        String::from_utf8(body).unwrap_or_else(|e| exitln!("Error: {:?}", e))
    }

    // depending on content type, do something
    match content_type.as_ref() {
        "application/json" => {
            Json::from_str(&read_str(body)).unwrap_or_else(|e| exitln!("Error: {:?}", e))
        },
        _ => match content_type.find("text/") {
            Some(idx) => match idx == 0 {
                true => {
                    Json::String(read_str(body))
                },
                false => as_base64(body)
            },
            _ => as_base64(body)
        }
    }

}
