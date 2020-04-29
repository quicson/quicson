# quicson

This is a simple [quic](https://tools.ietf.org/html/draft-ietf-quic-transport-27) client-server example written by 
[cloudflare](https://github.com/cloudflare/quiche/tree/master/examples) and modified on my own.

To run it type into `quicson` directory:

```bash
cargo run --bin server
cargo run --bin client https://127.0.0.1:4433
```

where: `https://127.0.0.1:4433` can be replaced by any other 
server address which supports quic protocol and is able to 
response for plain text messages.

### Requirements
To run this example you will need:
* [Go compiler](https://golang.org/dl/)
* g++ (on ubuntu based systems you can download and install it using apt)
* cmake (on ubuntu based systems you can download and install it using apt)