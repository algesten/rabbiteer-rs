use std::io::{self, Write};
use std::process;
use rand::{thread_rng, Rng};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::{self, Table, TableEntry};
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;
use output;

macro_rules! exitln(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
        process::exit(1);
    } }
);



pub fn do_subscribe(opts:amqp::Options, matches:&ArgMatches) {

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

        let msg = output::build_output(info, &deliver, &props, body);

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
                let outdir = Path::new(&output);
                let path = Path::new(&output).join(file_name);

                // don't allow writes outside this dir
                if outdir != path.parent().unwrap() {
                    exitln!("Error: fileName writes outside -o dir: {}", path.to_str().unwrap());
                }

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

fn gen_rand_name(ext:String) -> String {
    // generate 16 ascii chars
    let mut rand:String = thread_rng().gen_ascii_chars().take(16).collect();
    rand.push_str(".");
    rand.push_str(&ext);
    rand
}
