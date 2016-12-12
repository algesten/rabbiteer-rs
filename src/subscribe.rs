use std::io::{self, Write};
use rand::{thread_rng, Rng};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::{self, Table, TableEntry};
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;
use output;
use error::RbtError;


pub fn do_subscribe(opts:amqp::Options, matches:&ArgMatches) -> Result<(),RbtError> {

    let output = try!(value_t!(matches, "output", String));
    let info = matches.is_present("info");

    // type lookup map
    let types = try!(mime::Types::new().or(Err("Failed to read mime types")));

    // check output is a dir
    {
        if output != "-" {
            let meta = try!(fs::metadata(&output));
            if !meta.is_dir() {
                rbterr!("Output {} is not a directory", output);
            }
        }
    }

    let receive = move |deliver:Deliver, props:BasicProperties, body:Vec<u8>| -> Result<(),RbtError> {

        let msg = try!(output::build_output(info, &deliver, &props, body));

        match output.as_ref() {
            "-" => {

                // just write to stdout
                let stdout = io::stdout();

                // lock until end of scope
                let mut handle = stdout.lock();

                try!(handle.write(&msg));
                try!(handle.write(b"\n"));
                try!(handle.flush());

            },
            _   => {

                // extract file name from headers, or fall back on random
                let file_name = file_name_of(&props, &types);

                // path relative to output dir
                let outdir = Path::new(&output);
                let path = Path::new(&output).join(file_name);

                // don't allow writes outside this dir
                if outdir != path.parent().unwrap() {
                    rbterr!("Output {} is not a directory", output);
                }

                let mut f = try!(fs::File::create(path));
                try!(f.write_all(&msg));

            },
        }

        Ok(())

    };

    let receiver = client::Receiver {
        exchange: try!(value_t!(matches, "exchange", String)),
        routing_key: try!(value_t!(matches, "routing_key", String)),
        callback: Box::new(receive),
    };

    client::open_receive(opts, receiver)
}

fn file_name_of(props:&BasicProperties, types:&mime::Types) -> String {

    let content_type =
        props.content_type.clone().unwrap_or("application/octet-stream".to_owned());

    // figure out a good extension for this content type
    let ext = (|| {
        // we can get more than one filename extension for
        // a type, e.g. .jpg, .jpeg
        if let Some(x) = types.get_extension(&content_type) {
            if x.len() > 0 {
                return x[0].clone() // pick the first one
            }
        }
        "bin".to_owned()
    })();

    // prefer a fileName from headers, but fall back on
    // a random name.
    let headers = props.headers.clone().unwrap_or(Table::new());
    if let Some(entry) = headers.get("fileName") {
        if let TableEntry::LongString(ref f) = *entry {
            return f.clone();
        }
    }
    gen_rand_name(ext)
}

fn gen_rand_name(ext:String) -> String {
    // generate 16 ascii chars
    let mut rand:String = thread_rng().gen_ascii_chars().take(16).collect();
    rand.push_str(".");
    rand.push_str(&ext);
    rand
}
