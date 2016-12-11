use std::io::{self, Write};
use std::process;
use amqp;
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;


// helper function to turn a filename
// into a mime-type
fn type_from_file(file:&String) -> String {
    let t = mime::Types::new()
        .unwrap_or_else(|e| exitln!("Error: {:?}", e));
    let path = Path::new(&file);
    let mime = t.mime_for_path(&path);
    String::from(mime)
}


pub fn do_publish(opts:amqp::Options, matches:&ArgMatches) {

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
