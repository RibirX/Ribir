## Cli for ribir build
### SubCommand
**run-wasm**: build the example to wasm
1. compile to target wasm32-unknown-unknown
2. use wasm-bindgen to export relative function to js
3. serve the wasm in 127.0.0.1:8000 by simpl-http-server

you can see more usage information by --help.