## Cli for ribir
### SubCommand
#### run-wasm
build the example to wasm
1. Compile to target wasm32-unknown-unknown
2. Use wasm-bindgen to export relative function to js
3. Serve the wasm in 127.0.0.1:8000 by simpl-http-server

#### bundle
Bundle the native app
1. Change the directory to the package of the app
2. Create a config bundle.json in the app's package
3. Build the app by cargo.
4. Run the bundle command with the --config flag, followed by the path to the bundle.json file. For example: 
``` bash
cli bundle --config bundle.json
```
By default, this will bundle the release binary. To bundle the debug binary instead, use the --debug flag.

##### Bundle Config File Example
The bundle config file is in JSON format. Here is an simple example:
``` json
{
    "productName": "Example",
    "version": "1.0.0",
    "identifier": "com.example.app",
    "publisher": "Example Inc.",
    "homepage": "https://example.com/app",
    "icon": ["../Logo.ico"],
    "copyright": "Copyright (c) 2021 Example Inc.",
    "targets": ["Msi", "Nsis"],
}
```
Note that this is just an example, and the actual configuration will depend on the specific requirements of your application. and for more details, you can refer to the [`BundleConfig`] struct in the cli crate.
