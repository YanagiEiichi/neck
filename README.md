# Neck · [![LICENSE](https://img.shields.io/github/license/YanagiEiichi/neck)](LICENSE.txt) [![Unit Tests](https://github.com/YanagiEiichi/neck/actions/workflows/test.yml/badge.svg)](https://github.com/YanagiEiichi/neck/actions/workflows/test.yml)

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
      --tls                        Connect proxy server using TLS
      --tls-domain <TLS_DOMAIN>    Specify the domain for TLS, using the hostname of addr by default
  -h, --help                       Print help
```
