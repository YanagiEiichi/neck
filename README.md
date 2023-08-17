# Neck · [![LICENSE](https://img.shields.io/github/license/YanagiEiichi/neck)](LICENSE.txt) [![Unit Tests](https://github.com/YanagiEiichi/neck/actions/workflows/test.yml/badge.svg)](https://github.com/YanagiEiichi/neck/actions/workflows/test.yml)

A specialized HTTP proxy server used to traverse unidirectional network limitations.

Suppose you have two network zones: Zone A and Zone B.
Zone A cannot access Zone B, but Zone B can access Zone A.

Firstly, you need to set up servers in both Zone A and Zone B.
Deploy the Neck server in zone A, and deploy the Neck client in Zone B.

Once this is done, you will be able to access Zone B from Zone A through the Neck server.

## How does the Neck work?

### The Problem

You cannot access Zone B from zone A.

```mermaid
graph LR

subgraph B[Zone B]
  Goal
end

subgraph A[Zone A]
  You
end

You -- Broken --x B
B -- OK --> A
```

### The Solution of the Neck

Deploy the Neck server in zone A, and deploy the Neck client in Zone B.

```mermaid
graph TB

subgraph Zone B
  Goal
  Client[Neck Client]
end

subgraph Zone A
  You
  Server[Neck Server]
end

You --> Server
Client --> Server
Client --> Goal
```

### A complete sequence diagram

```mermaid
sequenceDiagram

actor You
participant Server as Server Neck
participant Client as Neck Client
participant Goal


Client ->> Server: Establish TCP,<br/>send a HTTP request to upgrade to "neck" protocol.
activate Client
activate Server
Server -->> Client: Respond with a status code of 101.

Note over Server,Client: This connection will not be closed here.<br/>It will be placed in a connection pool for future use.

You ->> Server: Establish TCP,<br/>send HTTP request via CONNECT method.
activate You
Server ->> Client: Retrieve a connection from the pool,<br/>and pass the request through it.
Client ->> Goal: Establish TCP with the address<br/>provided in a CONNECT request.
activate Goal
Goal -->> Client: Accept
Client -->> Server: Respond with a status code of 200,<br/>indicate the connection has established.
Server -->> You: Pass the response through.

Note over You, Goal: Now, the tunnel has been created. You can communicate directly with Goal.<br/>The middle Neck components just pass all data through.


deactivate You
deactivate Server
deactivate Client
deactivate Goal
```

### For Security

Neck uses HTTP, so it is not secure.
However, you can deploy the Neck Server behind a TLS load balancer to enhance its security.

```mermaid
graph TB

subgraph Zone B
  Goal
  Client[Neck Client]
end

subgraph Zone A
  You
  subgraph LB[TLS LB]
    Server[Neck Server]
  end
end

style LB fill:#0f02

You --Over TLS--> LB
Client --Over TLS--> LB
Client --> Goal
```

## Usage

### Server

```text
Start a Neck HTTP proxy server

Usage: neck serve [OPTIONS] [ADDR]

Arguments:
  [ADDR]  Binding the listening address defaults "0.0.0.0:1081"

Options:
      --max-workers <MAX_WORKERS>  The maximum allowed number of workers defaults 200
      --direct                     Proxy directly from the server without creating a worker pool
  -h, --help                       Print help
```

### Client

```text
Create some worker connections and join the pool of the server

Usage: neck join [OPTIONS] <ADDR>

Arguments:
  <ADDR>  Proxy server address

Options:
  -c, --connections <CONNECTIONS>  The number of maximum provided connections defaults 200
  -w, --workers <WORKERS>          The number of concurrent workers defaults 8
      --tls                        Connect proxy server using TLS
      --tls-domain <TLS_DOMAIN>    Specify the domain for TLS, using the hostname of addr by default
  -h, --help                       Print help
```

## Afterwords

> Across the Great Wall we can reach every corner in the world.
