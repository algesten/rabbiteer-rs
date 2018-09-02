use std::io::{self, Write};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
use amqp::protocol::basic::{Deliver, BasicProperties};
use amqp::{self, TableEntry, Channel};
use clap::ArgMatches;
use std::fs;
use std::path::Path;
use mime;
use client;
use output;
use error::RbtError;
use std::panic;


pub fn do_subscribe(opts:amqp::Options, matches:&ArgMatches) -> Result<(),RbtError> {

    let output = value_t!(matches, "output", String)?;
    let queue : Option<String> = matches.value_of("queue").map(str::to_owned);
    let force_declare : bool = matches.is_present("declare");
    let info   = matches.is_present("info");
    let single = matches.is_present("single");

    // type lookup map
    let types = mime::Types::new().or(Err("Failed to read mime types"))?;

    // check output is a dir
    {
        if output != "-" {
            let meta = fs::metadata(&output)?;
            if !meta.is_dir() {
                rbterr!("Output {} is not a directory", output);
            }
        }
    }

    let receive = move |channel: &mut Channel, deliver:Deliver, props:BasicProperties, body:Vec<u8>| ->
        Result<(),RbtError> {

        let msg = output::build_output(info, &deliver, &props, body)?;

        match output.as_ref() {
            "-" => {

                // just write to stdout
                let stdout = io::stdout();

                // lock until end of scope
                let mut handle = stdout.lock();

                handle.write(&msg)?;
                handle.write(b"\n")?;
                handle.flush()?;

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

                errln!("{}", path.to_str().unwrap());

                let mut f = fs::File::create(path)?;
                f.write_all(&msg)?;

            }

        }

        // maybe end here?
        if single {
            // closing the channel
            channel.close(200, "Bye")?;
            panic::set_hook(Box::new(|_| {
            }));
            // Until amqp library finds a way to exit consumer, terminate consumer_thread here.
            panic!();
        }

        Ok(())

    };

    let receiver = client::Receiver {
        exchange: value_t!(matches, "exchange", String)?,
        routing_key: matches.value_of("routing_key").map(str::to_owned),
        auto_ack: ! matches.is_present("noack"),
        callback: Box::new(receive),
    };

    client::open_receive(opts, queue, force_declare, receiver)
}

fn file_name_of(props:&BasicProperties, types:&mime::Types) -> String {

    let content_type =
        props.content_type.clone().unwrap_or("application/octet-stream".to_owned());

    // figure out a good extension for this content type
    let ext = types.get_extension(&content_type)
        .and_then(|x| Some(x[0].clone()))
        .or_else(|| Some("bin".to_owned()))
        .unwrap();

    // prefer a fileName from headers, but fall back on
    // a random name.
    props.headers.clone()
        .and_then(|x| match x.get("fileName") {
            Some(&TableEntry::LongString(ref f)) => Some((*f).clone()),
            _ => None
        })
        .or_else(|| Some(gen_rand_name(ext)))
        .unwrap()
}

fn gen_rand_name(ext:String) -> String {
    // generate 16 ascii chars
    let mut rand:String = thread_rng().sample_iter(&Alphanumeric).take(16).collect();
    rand.push_str(".");
    rand.push_str(&ext);
    rand
}
