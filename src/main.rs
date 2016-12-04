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

#[derive(Debug)]
enum Oper {
    Publish {
        exchange: String,
        routing_key: String,
        content_type: String,
        headers: Vec<String>,
        file: String,
    }
}

#[derive(Debug)]
struct Todo {
    host: String,
    port: u16,
    login: String,
    password: String,
    vhost: String,
    oper: Oper,
}

impl Todo {
    fn to_opts(&self) -> amqp::Options {
        amqp::Options {
            host:     self.host.clone(),
            port:     self.port.clone(),
            login:    self.login.clone(),
            password: self.password.clone(),
            vhost:    self.vhost.clone(),
            ..Default::default()
        }
    }
}

fn unwrap_publish(parent:&ArgMatches) -> Oper {
    let matches = parent.subcommand_matches("publish").unwrap();
    Oper::Publish {
        exchange:     value_t_or_exit!(matches.value_of("exchange"), String),
        routing_key:  value_t_or_exit!(matches.value_of("routing_key"), String),
        content_type: matches.value_of("content_type").unwrap_or("-").to_string(),
        headers:      values_t!(matches.values_of("header"), String).unwrap_or(vec![]),
        file:         value_t_or_exit!(matches.value_of("file"), String),
    }
}

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
        .get_matches();

    let todo = Todo {
        host:     value_t_or_exit!(matches.value_of("host"), String),
        port:     value_t_or_exit!(matches.value_of("port"), u16),
        login:    value_t_or_exit!(matches.value_of("login"), String),
        password: value_t_or_exit!(matches.value_of("password"), String),
        vhost:    value_t_or_exit!(matches.value_of("vhost"), String),
        oper:  match matches.subcommand_name() {
            Some("publish") => unwrap_publish(&matches),
            _ => exitln!("Error: Need subcommand. Try --help"),
        }
    };

    let opts = todo.to_opts();

    match todo.oper {
        Oper::Publish { exchange, routing_key, content_type, headers, file } => {
            let reader: Box<io::Read> = match file.as_ref() {
                "-" => Box::new(io::stdin()),
                _   => Box::new(fs::File::open(&file)
                                .unwrap_or_else(|e| exitln!("Error: {}", e))),
            };
            let content_type = match content_type.as_ref() {
                "-" => type_from_file(&file),
                _   => content_type,
            };
            let sendable = client::Sendable {
                exchange: exchange,
                routing_key: routing_key,
                content_type: content_type,
                headers: headers,
                reader: reader,
            };
            client::open_send(opts, sendable);
        }
    }

}
