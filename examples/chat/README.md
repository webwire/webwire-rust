# Chat example

This example implements a simple multi-server chat which uses
redis as message broker between the servers.

## Prerequisites

This example assumes a running redis installation at localhost.

## Start the server

```
$ copy .env.example .env
$ cargo run --bin server
Feb 03 01:03:54.377  INFO server: Listening on ws://127.0.0.1:2323
```

## Start the client

```
$ cargo run --bin client ws://127.0.0.1:2323
Feb 03 01:07:10.304  INFO client: Connecting to server ws://127.0.0.1:2323/
Feb 03 01:07:10.304  INFO client: Connected.
```

You can now type a message and press enter. It should appear both on the
server console and is echoed back to the client.

Now try starting multiple chat clients at the same time.

## Multiple servers

The easiest way is to provide the `LISTEN` environment variable
at startup.

```
$ LISTEN=127.0.0.1:2324 cargo run --bin server
Feb 03 01:12:26.871  INFO server: Listening on ws://127.0.0.1:2324
```

You can now connect to this server as before using the `client` binary:

```
$ cargo run --bin client ws://127.0.0.1:2324 bob
Feb 03 01:13:04.067  INFO client: Connecting to server ws://127.0.0.1:2324/
Feb 03 01:13:04.068  INFO client: Connected.
```

You should now be able to send messages from clients connected to port
2323 to clients connected to port 2324 and vice versa.
