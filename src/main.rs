extern crate rustc_serialize;
extern crate amqp;
extern crate rand;
extern crate conduit_mime_types as mime;
extern crate url;
#[macro_use] extern crate clap;

#[macro_use] mod error;
mod client;
mod output;
mod publish;
mod subscribe;

use std::env;
use std::fs;
use clap::{Arg, App, SubCommand};
use url::Url;
use rustc_serialize::json::Json;
use error::RbtError;


fn main() {
    _main().unwrap_or_else(error::handle);
}

static HOST:&'static str = "127.0.0.1";
static PORT:&'static str = "5672";
static USER:&'static str = "guest";
static PASS:&'static str = "guest";
static VHST:&'static str = "";

fn _main() -> Result<(),RbtError> {

    let matches = App::new("Rabbiteer")
        .version(crate_version!())
        .author("Martin Algesten <martin@algesten.se>")
        .about("Simple input/output tool for RabbitMQ")
        .arg(Arg::with_name("host")
             .help("RabbitMQ host")
             .short("h")
             .long("host")
             .takes_value(true)
             .default_value(HOST))
        .arg(Arg::with_name("port")
             .help("Port to connect to")
             .short("P")
             .long("port")
             .takes_value(true)
             .default_value(PORT))
        .arg(Arg::with_name("user")
             .help("User to authenticate with")
             .short("u")
             .alias("l")
             .long("user")
             .takes_value(true)
             .default_value(USER))
        .arg(Arg::with_name("password")
             .help("Password to authenticate with")
             .short("p")
             .long("password")
             .takes_value(true)
             .default_value(PASS))
        .arg(Arg::with_name("vhost")
             .help("Virtual host")
             .short("v")
             .long("vhost")
             .takes_value(true)
             .default_value(VHST))
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
                    .arg(Arg::with_name("rpc")
                         .help("Publish as RPC with replyTo and wait for reply.")
                         .long("rpc"))
                    .arg(Arg::with_name("content_type")
                         .help("Content type such as application/json. Inferred from filename if possible.")
                         .short("c")
                         .long("content-type")
                         .takes_value(true))
                    .arg(Arg::with_name("priority")
                         .help("Priority basic property")
                         .short("z")
                         .long("priority")
                         .takes_value(true))
                    .arg(Arg::with_name("rpctimeout")
                        .help("Rpc timeout in milliseconds")
                        .short("t")
                        .long("rpctimeout")
                        .default_value(""))
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
                         .takes_value(true))
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
                    .arg(Arg::with_name("single")
                         .help("Expect one single message, then quit.")
                         .short("1")
                         .long("single"))
                    .arg(Arg::with_name("queue")
                         .help("Use a named (non-auto_delete) queue.")
                         .takes_value(true)
                         .short("q")
                         .long("queue"))
                    .arg(Arg::with_name("declare")
                         .help("Force the declaration of a named queue. Default is to assume the queue is already declared")
                         .short("d")
                         .long("declare"))
                    .arg(Arg::with_name("noack")
                         .help("Do not automatically acknowledge received messages. (useful for peeking contents of an existing queue)")
                         .short("n")
                         .long("noack"))
        )
        .get_matches();

    // order of preference
    // 1. CONF-file
    // 2. URL
    // 3. explicit arg.

    // start with defaults.
    let mut opts = amqp::Options {
        host:     value_t!(matches, "host", String)?,
        port:     value_t!(matches, "port", u16)?,
        login:    value_t!(matches, "user", String)?,
        password: value_t!(matches, "password", String)?,
        vhost:    value_t!(matches, "vhost", String)?,
        ..Default::default()
    };

    // CONF file
    parse_conf(&mut opts);

    // URL arg
    if let Ok(urlstr) = value_t!(matches, "url", String) {
        parse_url(&mut opts, urlstr)?;
    }

    fn if_differs(opt:Option<&str>, def:&str, set:&mut FnMut(String) -> ()) {
        if let Some(v) = opt {
            if v != def {
                set(v.to_string());
            }
        }
    }

    if_differs(matches.value_of("host"), HOST, &mut|v|{ opts.host = v });
    if_differs(matches.value_of("port"), PORT, &mut|v|{ opts.port = v.parse::<u16>().unwrap() });
    if_differs(matches.value_of("user"), USER, &mut|v|{ opts.login = v });
    if_differs(matches.value_of("password"), PASS, &mut|v|{ opts.password = v });
    if_differs(matches.value_of("vhost"), VHST, &mut|v|{ opts.vhost = v });

    // depending on subcommand, we do one or the other
    match matches.subcommand_name() {

        // execute publish command
        Some("publish") => {

            // the args after the "publish command
            let subm = matches.subcommand_matches("publish").unwrap();

            publish::do_publish(opts, subm)

        },

        // execute subscribe command
        Some("subscribe") => {

            // the args after the "subscribe" command
            let subm = matches.subcommand_matches("subscribe").unwrap();

            subscribe::do_subscribe(opts, subm)

        },

        _ => rbterr!("Need subcommand. Try --help"),
    }

}



// update the opts object with the given url
fn parse_url(opts:&mut amqp::Options, urlstr:String) -> Result<(),RbtError> {
    if let Ok(url) = Url::parse(urlstr.as_ref()) {
        if url.scheme() != "amqp" {
            rbterr!("Unknown scheme: {}", url);
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
        Ok(())
    } else {
        rbterr!("Unable to parse url: {}", urlstr);
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
