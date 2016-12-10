Rabbiteer for Rust
==================

CLI tool for publishing and subscribing to RabbitMQ.

## Install

Install Cargo (Rust).

```bash
$ brew install rust
```

Install rabbiteer

```bash
$ cargo install rabbiteer
```

## Command format

The command has two "modes", `publish` and `subscribe`.

```bash
$ rabbiteer [general opts] publish/subscribe [specific opts]
```

### General options

```bash
$ rabbiteer --help
...
OPTIONS:
    -h, --host <host>            RabbitMQ host [default: 127.0.0.1]
    -p, --password <password>    Password to authenticate with [default: guest]
        --port <port>            Port to connect to [default: 5672]
    -u, --user <user>            Username to authenticate with [default: guest]
    -v, --vhost <vhost>          Virtual host [default: guest]
```

## Publish

Publishing pushes data from stdin or a file to an exchange.

```bash
$ rabbiteer publish --help
...
OPTIONS:
    -c, --content-type <content_type>    Content type such as application/json. Inferred from filename if
                                         possible.
    -e, --exchange <exchange>            Exchange to publish to
    -f, --file <file>                    Filename (- is stdin) [default: -]
    -H, --header <header>...             Header on the form "My-Header: Value"
    -r, --routing-key <routing_key>      Routing key [default: ]
```


### Example

#### From stdin

Provide the content-type as arg.

```bash
$ echo "{\"panda\":true}" | \
    rabbiteer -u admin -p admin -v prod publish -e myexchange -c application/json
```

#### From a file

Content-type is inferred if possible.

```
$ rabbiteer -u admin -p admin -v prod publish -e myexchange -H "batch: true" -f ./foo.json
```


## Subscribe

Subscribing binds an anonymous queue to an exchange and listens to
messages pushed to that exchange. The messages can either be dumped to
stdout or as files to a directory.

```bash
$ rabbiteer subscribe --help
...
FLAGS:
    -i, --info       Include delivery info (and headers).
OPTIONS:
    -e, --exchange <exchange>          Exchange to subscribe to
    -o, --output <output>              Output directory (- is stdout) [default: -]
    -r, --routing-key <routing_key>    Routing key [default: #]
```

### Example

#### To stdout

Outputs the body of each message to stdout. Makes no conversion
of the body. If the body is binary, you will see gibberish in the
terminal.

```bash
$ rabbiteer -u admin -p admin -v prod subscribe -e myexchange
...
```

##### With delivery info

`-i` means we make a JSON wrapping of the message so we can include
some basic information about the delivery.

This always produces a JSON structure. When the body is binary (as
indicated by `content_type`), the data is base64 encoded and when the
type is `text/...`, we output it as a JSON string.

The format is:

```json
{
    "deliver":{
        "consumer_tag":"amq.ctag-Tjxx5Qy5zAr0o_yiLOsNEg",
        "delivery_tag":1,
        "redelivered":false,
        "exchange":"myexchange",
        "routing_key":"text"
    },
    "props":{
        "content_type":"application/json",
        "headers":{
            "backendUpdate":false,
            "batch":false,
            "index":"dist-text",
            "oper":"index"
        }
    },
    "data": <body>
}
```

```bash
$ rabbiteer -u admin -p admin -v prod subscribe -e myexchange -i
...
```

#### To a directory

With `-o` the body of each individual message is output to a separate
file. The header `fileName` can be supplied by the sender, in which case
that file is (over-)written.

```bash
$ rabbiteer -u admin -p admin -v prod subscribe -e myexchange -o /tmp
...
```
