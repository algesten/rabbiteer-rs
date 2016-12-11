use rustc_serialize::json::{self, Json, Object};
use rustc_serialize::base64::{self, ToBase64};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::TableEntry;
use error::RbtError;


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

pub fn build_output(info:bool, deliver:&Deliver,
                    props:&BasicProperties, body:Vec<u8>) -> Result<Vec<u8>,RbtError> {
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
                let val = try!(entry_to_json(entry));
                mprops.headers.insert(key, val);
            }
        }

        // the body
        let data = try!(figure_out_body(content_type, body));

        // and put it together
        let msg = Msg {
            deliver:mdel,
            props:mprops,
            data:data,
        };

        // encode
        let js = json::as_pretty_json(&msg);

        // convert to bytes
        Ok(js.to_string().as_bytes().to_owned())

    } else {

        let content_type = props.content_type.clone().unwrap_or(String::from(""));

        match content_type.as_ref() {
            "application/json" => {
                // interpret body so we can pretty print it
                let body = try!(figure_out_body(content_type, body));

                // encode back as pretty
                let js = json::as_pretty_json(&body);

                // convert to bytes
                Ok(js.to_string().as_bytes().to_owned())
            },

            // just return untranslated bytes
            _ => Ok(body)
        }

    }
}


fn figure_out_body(content_type:String, body:Vec<u8>) -> Result<Json,RbtError> {

    // depending on content type, do something
    match content_type.as_ref() {
        "application/json" => Ok(try!(Json::from_str(&try!(String::from_utf8(body))))),
        _ => {
            if let Some(_) = content_type.find("text/") {
                return Ok(Json::String(try!(String::from_utf8(body))));
            } else {
                Ok(Json::String(body.to_base64(base64::STANDARD)))
            }
        }
    }

}


fn entry_to_json(entry:TableEntry) -> Result<Json,String> {
    match entry {
        TableEntry::Bool(v)           => Ok(Json::Boolean(v)),
        TableEntry::ShortShortInt(v)  => Ok(Json::I64(v as i64)),
        TableEntry::ShortShortUint(v) => Ok(Json::U64(v as u64)),
        TableEntry::ShortInt(v)       => Ok(Json::I64(v as i64)),
        TableEntry::ShortUint(v)      => Ok(Json::U64(v as u64)),
        TableEntry::LongInt(v)        => Ok(Json::I64(v as i64)),
        TableEntry::LongUint(v)       => Ok(Json::U64(v as u64)),
        TableEntry::LongLongInt(v)    => Ok(Json::I64(v)),
        TableEntry::LongLongUint(v)   => Ok(Json::U64(v)),
        TableEntry::Float(v)          => Ok(Json::F64(v as f64)),
        TableEntry::Double(v)         => Ok(Json::F64(v)),
        TableEntry::LongString(v)     => Ok(Json::String(v)),
        TableEntry::Void              => Ok(Json::Null),
        _                             => Err(format!("Cant translate {:?}", entry)),
        // ShortString(String),
        // TableEntry::FieldTable(Table) =>
        // TableEntry::Timestamp(u64) =>
        // TableEntry::FieldArray(Vec<TableEntry>) =>
        // TableEntry::DecimalValue(u8, u32) => mprops.headers.insert(key, v),
    }
}
