# Neck

A specialized HTTP proxy server used to traverse unidirectional network limitations.

## Usage

Suppose you have two network groups: A and B.
Group A can access B, but group B cannot access A.

First, you must set up two servers in A and B network groups.
Deploy the Neck server to group B, and deploy the Neck client to group A.

Afterward, you can access A group from B via Neck server.

### Server

```text
Start a Neck HTTP proxy server

Usage: neck serve [ADDR]

Arguments:
  [ADDR]  Binding the listening address defaults "0.0.0.0:1081"

Options:
  -h, --help  Print help
```

### Client

```text
Create some worker connections and join the pool of the server

Usage: neck join [OPTIONS] <ADDR>

Arguments:
  <ADDR>  Proxy server address

Options:
  -c, --connections <CONNECTIONS>  The provided connections defaults 100
  -h, --help                       Print help
```
