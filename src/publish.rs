use std::io::{self, Write};
use amqp;
use amqp::protocol::basic::{Deliver, BasicProperties};
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;
use error::RbtError;
use output;


// helper function to turn a filename
// into a mime-type
fn type_from_file(file:&String) -> Result<String,RbtError> {
    let t = mime::Types::new().or(Err("Failed to read mime types"))?;
    let path = Path::new(&file);
    let mime = t.mime_for_path(&path);
    Ok(mime.to_owned())
}


pub fn do_publish(opts:amqp::Options, matches:&ArgMatches) -> Result<(),RbtError> {

    // either stdin or a file
    let file = value_t!(matches, "file", String)?;
    let rpc  = matches.is_present("rpc");
    let reader: Box<io::Read> = match file.as_ref() {
        "-" => Box::new(io::stdin()),
        _   => Box::new(fs::File::open(&file)?),
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
            "-" => type_from_file(&file)?,
            _   => c,
        }
    };

    // the sendable wraps up the parsed parts
    let sendable = client::Sendable {
        exchange:     value_t!(matches, "exchange", String)?,
        routing_key:  value_t!(matches, "routing_key", String)?,
        content_type: content_type,
        headers:      values_t!(matches, "header", String).unwrap_or(vec![]),
        file_name:    file_name.to_owned(),
        reader:       reader,
        priority:     value_t!(matches, "priority", u8).unwrap_or(0),
        rpctimeout:   value_t!(matches, "rpctimeout", u64).unwrap_or(0)
    };

    // if we are doing an rpc with replyTo/correlationId, we set up a receiver
    let rpc_receive = match rpc {
        false => None,
        true  => {
            let receive =
                move |deliver:Deliver, props:BasicProperties, body:Vec<u8>| ->
                Result<(),RbtError> {
                    let msg = output::build_output(false, &deliver, &props, body)?;

                    // just write to stdout
                    let stdout = io::stdout();

                    // lock until end of scope
                    let mut handle = stdout.lock();

                    handle.write(&msg)?;
                    handle.write(b"\n")?;
                    handle.flush()?;

                    // it's the end
                    ::std::process::exit(0)
                };

            let receiver = client::Receiver {
                exchange: "".to_owned(),
                routing_key: "".to_owned(),
                callback: Box::new(receive),
            };

            Some(receiver)
        }
    };

    // ship it
    client::open_send(opts, sendable, rpc_receive)
}
