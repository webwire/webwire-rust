# Chat example

Run the example via `cargo run`.

Now the server should be listening at port 2323.

Using `websocat` you should be able to send a request to the server:

```
$ websocat ws://localhost:2323
2 1 Server.send {"text": "Hello world!"}
```

The server should be responding with the following data:

```
1 1 Client.on_message {"text":"Hello world!"}
3 2 1 {"Ok":null}
```

First it tries to notify the `Client.on_message` service function
and then confirms the `Server.send` request with `Ok(())`.

You can try connecting with multiple `websocat` commands in parallel
and see a working albeit simple chat service.
