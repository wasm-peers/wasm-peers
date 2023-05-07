# wasm-peers-signaling-server

This crate provides a [signaling server](https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Signaling_and_video_calling#the_signaling_server)
implementation compatible with [wasm-peers](https://docs.rs/wasm-peers/latest/wasm_peers/) crate.

To learn more, check out main [README](https://github.com/wasm-peers/wasm-peers#readme)
of the project.

## Usage

The server is available on crates.io and installable with `cargo install`.

Just run:

```bash
cargo install wasm-peers-signaling-server
# by default server runs on 127.0.0.1:9001
wasm-peers-signaling-server 0.0.0.0:9001
```

Alternatively theres is a `Dockerfile` in the repo root, and available on [Docker Hub](https://hub.docker.com/r/tomkarw/wasm-peers-signaling-server).

Now you can take the public IP address of the server and provide it to an instance of network manager from the main crate.

This server provides 3 endpoints, which one you should use depends on the chosen topology:

* `ws://<ip-address>:<port>/one-to-one` - for [one-to-one](https://docs.rs/wasm-peers/latest/wasm_peers/one_to_one/index.html) connections.
* `ws://<ip-address>:<port>/one-to-many` - for [one-to-many](https://docs.rs/wasm-peers/latest/wasm_peers/one_to_many/index.html) connections.
* `ws://<ip-address>:<port>/many-to-many` - for [many-to-many](https://docs.rs/wasm-peers/latest/wasm_peers/many_to_many/index.html) connections.
