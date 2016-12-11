extern crate rustc_serialize;
extern crate amqp;
extern crate rand;
extern crate conduit_mime_types as mime;
extern crate url;
#[macro_use] extern crate clap;

use std::io::Write;
use std::process;
use std::env;
use std::fs;
use clap::{Arg, App, SubCommand};
use url::Url;
use rustc_serialize::json::Json;

mod client;
mod output;
mod publish;
mod subscribe;

macro_rules! errln(
    ($($arg:tt)*) => { {
        writeln!(&mut ::std::io::stderr(), $($arg)*).expect("failed printing to stderr");
    } }
);

macro_rules! exitln(
    ($($arg:tt)*) => { {
        errln!($($arg)*);
        process::exit(1);
    } }
);




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
             .short("P")
             .long("port")
             .takes_value(true)
             .default_value("5672"))
        .arg(Arg::with_name("user")
             .help("User to authenticate with")
             .short("u")
             .alias("l")
             .long("user")
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
             .default_value(""))
        .arg(Arg::with_name("url")
             .help("AMQP connection url (amqp://user:pass@host:port/vhost)")
             .short("U")
             .long("url")
             .takes_value(true))
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
                         .default_value("#"))
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
    let mut opts = amqp::Options {
        host:     value_t_or_exit!(matches.value_of("host"), String),
        port:     value_t_or_exit!(matches.value_of("port"), u16),
        login:    value_t_or_exit!(matches.value_of("user"), String),
        password: value_t_or_exit!(matches.value_of("password"), String),
        vhost:    value_t_or_exit!(matches.value_of("vhost"), String),
        ..Default::default()
    };

    // url overrides the defaults
    if let Ok(urlstr) = value_t!(matches.value_of("url"), String) {
        parse_url(&mut opts, urlstr);
    } else {
        // we use the CONF env first, and if
        // that doesn't work out, we fall back on
        // RABBITEER_URL
        if !parse_conf(&mut opts) {
            if let Ok(urlstr) = env::var("RABBITEER_URL") {
                parse_url(&mut opts, urlstr);
            }
        }
    }

    errln!("Connecting to amqp://{}:{}@{}:{}/{}",
           opts.login, opts.password, opts.host, opts.port, opts.vhost);

    // depending on subcommand, we do one or the other
    match matches.subcommand_name() {

        // execute publish command
        Some("publish") => {

            // the args after the "publish command
            let subm = matches.subcommand_matches("publish").unwrap();

            publish::do_publish(opts, subm);

        },

        // execute subscribe command
        Some("subscribe") => {

            // the args after the "subscribe" command
            let subm = matches.subcommand_matches("subscribe").unwrap();

            subscribe::do_subscribe(opts, subm);

        },

        _ => exitln!("Error: Need subcommand. Try --help"),
    };

}



// update the opts object with the given url
fn parse_url(opts:&mut amqp::Options, urlstr:String) {
    if let Ok(url) = Url::parse(urlstr.as_ref()) {
        if url.scheme() != "amqp" {
            exitln!("Unknown scheme: {}", url);
        }
        if let Some(host) = url.host_str() {
            opts.host = host.to_owned();
        }
        if let Some(port) = url.port() {
            opts.port = port;
        }
        if url.username() != "" {
            opts.login = url.username().to_owned();
        }
        if let Some(password) = url.password() {
            opts.password = password.to_owned();
        }
        if let Some(mut segs) = url.path_segments() {
            if let Some(vhost) = segs.nth(0) {
                opts.vhost = vhost.to_owned();
            }
        }
    } else {
        exitln!("Unable to parse url: {}", urlstr);
    }
}



// update the opts object with the conf
fn parse_conf(opts:&mut amqp::Options) -> bool {

    let mut update = |connopt:Option<&Json>| -> bool {
        if let Some(conn) = connopt {
            if conn.is_object() {
                if let Json::String(ref v) = conn["host"] {
                    opts.host = v.to_owned();
                }
                if let Json::String(ref v) = conn["vhost"] {
                    opts.vhost = v.to_owned();
                }
                if let Json::String(ref v) = conn["login"] {
                    opts.login = v.to_owned();
                }
                if let Json::String(ref v) = conn["password"] {
                    opts.password = v.to_owned();
                }
                return true;
            }
        }
        false
    };

    if let Ok(file) = env::var("CONF") {
        if let Ok(mut reader) = fs::File::open(&file) {
            if let Ok(conf) = Json::from_reader(&mut reader) {
                let first = &["amqp", "connection"];
                let second = &["amqp"];
                if update(conf.find_path(first)) || update(conf.find_path(second)) {
                    return true;
                }
            }
        }
    }
    false
}
