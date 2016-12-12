use std::io;
use amqp;
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;
use error::RbtError;


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
    };

    // ship it
    client::open_send(opts, sendable)
}
