use std::path::Path;
use std::io::{self, Write};
use std::process;
use std::fs;

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

extern crate amqp;
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::{Table, TableEntry};

mod client;

extern crate conduit_mime_types as mime;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
        process::exit(1);
    } }
);

// helper function to turn a filename
// into a mime-type
fn type_from_file(file:&String) -> String {
    let t = mime::Types::new()
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    let path = Path::new(&file);
    let mime = t.mime_for_path(&path);
    String::from(mime)
}


fn main() {

    let matches = App::new("Rabbiteer")
        .version(crate_version!())
        .author("Martin Algesten <martin@algesten.se>")
        .about("Simple input/output tool for RabbitMQ")
        .arg(Arg::with_name("host")
             .help("RabbitMQ host")
             .short("h")
             .long("host")
             .takes_value(true)
             .default_value("127.0.0.1"))
        .arg(Arg::with_name("port")
             .help("Port to connect to")
             .long("port")
             .takes_value(true)
             .default_value("5672"))
        .arg(Arg::with_name("login")
             .help("Login to authenticate with")
             .short("l")
             .long("login")
             .takes_value(true)
             .default_value("guest"))
        .arg(Arg::with_name("password")
             .help("Password to authenticate with")
             .short("p")
             .long("password")
             .takes_value(true)
             .default_value("guest"))
        .arg(Arg::with_name("vhost")
             .help("Virtual host")
             .short("v")
             .long("vhost")
             .takes_value(true)
             .default_value("guest"))
        .subcommand(SubCommand::with_name("publish")
                    .about("Publish data to an exchange")
                    .arg(Arg::with_name("exchange")
                         .help("Exchange to publish to")
                         .short("e")
                         .long("exchange")
                         .takes_value(true)
                         .required(true))
                    .arg(Arg::with_name("routing_key")
                         .help("Routing key")
                         .short("r")
                         .long("routing-key")
                         .takes_value(true)
                         .default_value(""))
                    .arg(Arg::with_name("header")
                         .help("Header on the form \"My-Header: Value\"")
                         .short("H")
                         .long("header")
                         .takes_value(true)
                         .number_of_values(1)
                         .multiple(true))
                    .arg(Arg::with_name("file")
                         .help("Filename (- is stdin)")
                         .short("f")
                         .long("file")
                         .takes_value(true)
                         .default_value("-"))
                    .arg(Arg::with_name("content_type")
                         .help("Content type such as application/json. Inferred from filename if possible.")
                         .short("c")
                         .long("content-type")
                         .takes_value(true))
        )
        .subcommand(SubCommand::with_name("subscribe")
                    .about("Subscribe to an exchange")
                    .arg(Arg::with_name("exchange")
                         .help("Exchange to subscribe to")
                         .short("e")
                         .long("exchange")
                         .takes_value(true)
                         .required(true))
                    .arg(Arg::with_name("routing_key")
                         .help("Routing key")
                         .short("r")
                         .long("routing-key")
                         .takes_value(true)
                         .default_value(""))
                    .arg(Arg::with_name("output")
                         .help("Output directory (- is stdout)")
                         .short("o")
                         .long("output")
                         .takes_value(true)
                         .default_value("-"))
                    .arg(Arg::with_name("info")
                         .help("Include delivery info (and headers).")
                         .short("i")
                         .long("info"))
        )
        .get_matches();

    // global opts, before "publish or "subscribe"
    let opts = amqp::Options {
        host:     value_t_or_exit!(matches.value_of("host"), String),
        port:     value_t_or_exit!(matches.value_of("port"), u16),
        login:    value_t_or_exit!(matches.value_of("login"), String),
        password: value_t_or_exit!(matches.value_of("password"), String),
        vhost:    value_t_or_exit!(matches.value_of("vhost"), String),
        ..Default::default()
    };

    // depending on subcommand, we do one or the other
    match matches.subcommand_name() {

        // execute publish command
        Some("publish") => {

            // the args after the "publish command
            let subm = matches.subcommand_matches("publish").unwrap();

            do_publish(opts, subm);

        },

        // execute subscribe command
        Some("subscribe") => {

            // the args after the "subscribe" command
            let subm = matches.subcommand_matches("subscribe").unwrap();

            do_subscribe(opts, subm);

        },

        _ => exitln!("Error: Need subcommand. Try --help"),
    };

}

fn do_publish(opts:amqp::Options, matches:&ArgMatches) {

    // either stdin or a file
    let file = value_t_or_exit!(matches.value_of("file"), String);
    let reader: Box<io::Read> = match file.as_ref() {
        "-" => Box::new(io::stdin()),
        _   => Box::new(fs::File::open(&file)
                        .unwrap_or_else(|e| exitln!("Error: {}", e))),
    };

    // either - or the name of the file
    let file_name:&str = match file.as_ref() {
        "-" => "-",
        _   => {
            // XXX fix unwrapping
            let ostr = Path::new(&file).file_name().unwrap();
            ostr.to_str().unwrap()
        }
    };

    // figure out a good content type
    // XXX should we fall back on binary octet-stream?
    let content_type = {
        let c = matches.value_of("content_type").unwrap_or("-").to_string();
        match c.as_ref() {
            "-" => type_from_file(&file),
            _   => c,
        }
    };

    // the sendable wraps up the parsed parts
    let sendable = client::Sendable {
        exchange: value_t_or_exit!(matches.value_of("exchange"), String),
        routing_key: value_t_or_exit!(matches.value_of("routing_key"), String),
        content_type: content_type,
        headers: values_t!(matches.values_of("header"), String).unwrap_or(vec![]),
        file_name: file_name.to_owned(),
        reader: reader,
    };

    // ship it
    client::open_send(opts, sendable);

}


extern crate rand;
use rand::{thread_rng, Rng};

fn gen_rand_name(ext:String) -> String {
    // generate 16 ascii chars
    let mut rand:String = thread_rng().gen_ascii_chars().take(16).collect();
    rand.push_str(".");
    rand.push_str(&ext);
    rand
}

fn file_name_of(props:&BasicProperties, types:&mime::Types) -> String {

    let content_type =
        props.content_type.clone().unwrap_or("application/octet-stream".to_owned());

    // figure out a good extension for this content type
    let ext = match types.get_extension(&content_type) {
        None => "bin".to_owned(),
        Some(x) => {
            // we can get more than one filename extension for
            // a type, e.g. .jpg, .jpeg
            match x.len() {
                0 => "bin".to_owned(),
                _ => {
                    x[0].clone() // pick the first one
                }
            }
        },
    };

    // prefer a fileName from headers, but fall back on
    // a random name.
    let headers = props.headers.clone().unwrap_or(Table::new());
    if headers.contains_key("fileName") {
        match headers.get("fileName").unwrap() {
            &TableEntry::LongString(ref f) => f.clone(),
            _ => gen_rand_name(ext),
        }
    } else {
        gen_rand_name(ext)
    }

}

fn do_subscribe(opts:amqp::Options, matches:&ArgMatches) {

    let output = value_t_or_exit!(matches.value_of("output"), String);
    let info = matches.is_present("info");

    // type lookup map
    let types = mime::Types::new()
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));

    // check output is a dir
    {
        if output != "-" {
            let meta = fs::metadata(&output)
                .unwrap_or_else(|e| exitln!("Error: {:?}", e));
            if !meta.is_dir() {
                exitln!("Output {} is not a directory", output);
            }
        }
    }

    let receive = move |deliver:Deliver, props:BasicProperties, body:Vec<u8>| {

        let msg = build_output(info, &deliver, &props, body);

        match output.as_ref() {
            "-" => {

                // just write to stdout
                let stdout = io::stdout();

                // lock until end of scope
                let mut handle = stdout.lock();

                handle.write(&msg)
                    .unwrap_or_else(|e| exitln!("Error: {:?}", e));
                handle.write(b"\n")
                    .unwrap_or_else(|e| exitln!("Error: {:?}", e));
                handle.flush()
                    .unwrap_or_else(|e| exitln!("Error: {:?}", e));

            },
            _   => {

                // extract file name from headers, or fall back on random
                let file_name = file_name_of(&props, &types);

                // path relative to output dir
                let path = Path::new(&output).join(file_name);

                let mut f = fs::File::create(path).expect("Unable to create file");
                f.write_all(&msg).expect("Unable to write data");

            },
        }

    };

    let receiver = client::Receiver {
        exchange: value_t_or_exit!(matches.value_of("exchange"), String),
        routing_key: value_t_or_exit!(matches.value_of("routing_key"), String),
        callback: Box::new(receive),
    };

    client::open_receive(opts, receiver);

}

extern crate rustc_serialize;
use rustc_serialize::json::{self, Json, Object};

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

fn build_output(info:bool, deliver:&Deliver, props:&BasicProperties, body:Vec<u8>) -> Vec<u8> {
    if info {

        // delivery info
        let mdel = MsgDeliver {
            consumer_tag:deliver.consumer_tag.clone(),
            delivery_tag:deliver.delivery_tag.clone(),
            redelivered:deliver.redelivered.clone(),
            exchange:deliver.exchange.clone(),
            routing_key:deliver.routing_key.clone(),
        };

        let content_type = props.clone().content_type.unwrap_or(String::from(""));

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
        let js = json::encode(&msg).unwrap();

        // convert to bytes
        js.to_string().as_bytes().to_owned()

    } else {

        body

    }
}


fn figure_out_body(content_type:String,body:Vec<u8>) -> Json {

    // interpret body as a string
    let jstr = || -> String {
        String::from_utf8(body)
            .unwrap_or_else(|e| exitln!("Error: {:?}", e))
    };

    // interpret body as a binary and base64 encode
    let as_base64 = move || -> Json {
        Json::String(String::from("asbase64"))
    };

    // depending on content type, do something
    match content_type.as_ref() {
        "application/json" => Json::from_str(&jstr())
            .unwrap_or_else(|e| exitln!("Error: {:?}", e)),
        _ => match content_type.find("text/") {
            Some(idx) => match idx == 0 {
                true => Json::String(String::from(jstr())),
                false => as_base64()
            },
            _ => as_base64()
        }
    }

}
