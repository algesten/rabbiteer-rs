use std::path::Path;
use std::io::Write;
use std::process;
use std::fs;
use std::io;

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

extern crate amqp;

mod client;

extern crate conduit_mime_types as mime;
use mime::Types;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
        process::exit(1);
    } }
);

fn type_from_file(file:&String) -> String {
    let t = Types::new()
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
                         .takes_value(true)
                    )
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
                         .default_value("-")
                    )
        )
        .get_matches();

    fn to_opts(matches:&ArgMatches) -> amqp::Options {
        amqp::Options {
            host:     value_t_or_exit!(matches.value_of("host"), String),
            port:     value_t_or_exit!(matches.value_of("port"), u16),
            login:    value_t_or_exit!(matches.value_of("login"), String),
            password: value_t_or_exit!(matches.value_of("password"), String),
            vhost:    value_t_or_exit!(matches.value_of("vhost"), String),
            ..Default::default()
        }
    }

    // global opts, before "publish or "subscribe"
    let opts = to_opts(&matches);

    match matches.subcommand_name() {

        // execute publish comman
        Some("publish") => {

            // after the command
            let subm = matches.subcommand_matches("publish").unwrap();

            // either stdin or a file
            let file = value_t_or_exit!(subm.value_of("file"), String);
            let reader: Box<io::Read> = {
                match file.as_ref() {
                    "-" => Box::new(io::stdin()),
                    _   => Box::new(fs::File::open(&file)
                                    .unwrap_or_else(|e| exitln!("Error: {}", e))),
                }
            };

            // figure out a good content type
            // XXX should we fall back on binary octet-stream?
            let content_type = {
                let c = subm.value_of("content_type").unwrap_or("-").to_string();
                match c.as_ref() {
                    "-" => type_from_file(&file),
                    _   => c,
                }
            };

            // the sendable wraps up the parsed parts
            let sendable = client::Sendable {
                exchange: value_t_or_exit!(subm.value_of("exchange"), String),
                routing_key: value_t_or_exit!(subm.value_of("routing_key"), String),
                content_type: content_type,
                headers: values_t!(subm.values_of("header"), String).unwrap_or(vec![]),
                reader: reader,
            };

            // ship it
            client::open_send(opts, sendable);
        },

        Some("subscribe") => {

            let subm = matches.subcommand_matches("subscribe").unwrap();
            let exchange = value_t_or_exit!(subm.value_of("exchange"), String);
            let routing_key = value_t_or_exit!(subm.value_of("routing_key"), String);
            let output = value_t_or_exit!(subm.value_of("output"), String);

        },

        _ => exitln!("Error: Need subcommand. Try --help"),
    };

}
