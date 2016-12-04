Rabbiteer for Rust
==================

It's early days, the tool can publish.

### General opts

```
$ rabbiteer --help
Rabbiteer 0.1.0
Martin Algesten <martin@algesten.se>
Simple input/output tool for RabbitMQ

USAGE:
    rabbiteer [OPTIONS] [SUBCOMMAND]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --host <host>            RabbitMQ host [default: 127.0.0.1]
    -l, --login <login>          Login to authenticate with [default: guest]
    -p, --password <password>    Password to authenticate with [default: guest]
        --port <port>            Port to connect to [default: 5672]
    -v, --vhost <vhost>          Virtual host [default: guest]

SUBCOMMANDS:
    help       Prints this message or the help of the given subcommand(s)
    publish    Publish data to an exchange
```

### Publish

```
$ rabbiteer publish --help
rabbiteer-publish 
Publish data to an exchange

USAGE:
    rabbiteer publish [OPTIONS] --exchange <exchange>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --content-type <content_type>    Content type such as application/json. Inferred from filename if possible.
    -e, --exchange <exchange>            Exchange to publish to
    -f, --file <file>                    Filename (- is stdin) [default: -]
    -H, --header <header>...             Header on the form "My-Header: Value"
    -r, --routing-key <routing_key>      Routing key [default: ]
```

### Example

```
$ rabbiteer -l admin -p admin -v prod publish -e jsonin -H "batch: true" -f ./foo.json
```
