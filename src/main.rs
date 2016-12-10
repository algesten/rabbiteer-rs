extern crate rustc_serialize;
extern crate amqp;
extern crate rand;
extern crate conduit_mime_types as mime;

use std::io::Write;
use std::process;

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand};

mod client;
mod output;
mod publish;
mod subscribe;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
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
    let opts = amqp::Options {
        host:     value_t_or_exit!(matches.value_of("host"), String),
        port:     value_t_or_exit!(matches.value_of("port"), u16),
        login:    value_t_or_exit!(matches.value_of("user"), String),
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
