use std::process;

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

struct Publish {
}

#[derive(Debug)]
enum Oper {
    Publish {
        exchange: String,
        routing_key: String,
        file: String,
        content_type: String,
    }
}

#[derive(Debug)]
struct Todo {
    host: String,
    port: u16,
    user: String,
    pass: String,
    oper: Oper,
}

fn main() {

    let mut app = App::new("Rabbiteer")
        .version(crate_version!())
        .author("Martin Algesten <martin@algesten.se>")
        .about("Simple input/output tool for RabbitMQ")
        .arg(Arg::with_name("host")
             .help("RabbitMQ host")
             .short("h")
             .long("host")
             .takes_value(true)
             .default_value("localhost"))
        .arg(Arg::with_name("port")
             .help("Port to connect to")
             .long("port")
             .takes_value(true)
             .default_value("5672"))
        .arg(Arg::with_name("user")
             .help("Username to authenticate with")
             .short("u")
             .long("user")
             .takes_value(true)
             .default_value("admin"))
        .arg(Arg::with_name("pass")
             .help("Password to authenticate with")
             .short("p")
             .long("pass")
             .takes_value(true)
             .default_value("admin"))
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
                    .arg(Arg::with_name("file")
                         .help("Filename (- is stdin)")
                         .short("f")
                         .long("file")
                         .takes_value(true)
                         .default_value("-"))
                    .arg(Arg::with_name("content_type")
                         .help("Content type such as text/json.
                               Inferred from filename if possible.")
                         .short("c")
                         .long("content-type")
                         .takes_value(true)
                    )
        );

    let matches = app.get_matches();

    let unwrap_publish = |parent:&ArgMatches| {
        let matches = parent.subcommand_matches("publish").unwrap();
        return Oper::Publish {
            exchange:     value_t_or_exit!(matches.value_of("exchange"), String),
            routing_key:  value_t_or_exit!(matches.value_of("routing_key"), String),
            file:         value_t_or_exit!(matches.value_of("file"), String),
            content_type: matches.value_of("content_type").unwrap_or("-").to_string(),
        }
    };

   let todo = Todo {
        host: value_t_or_exit!(matches.value_of("host"), String),
        port: value_t_or_exit!(matches.value_of("port"), u16),
        user: value_t_or_exit!(matches.value_of("user"), String),
        pass: value_t_or_exit!(matches.value_of("pass"), String),
        oper:  match matches.subcommand_name() {
            Some("publish") => unwrap_publish(&matches),
            _ => {
                println!("Unrecognized subcommand: {}", matches.subcommand_name().unwrap_or("<None>"));
                process::exit(1);
            },
        }
    };

    println!("{:?}", todo)

}
