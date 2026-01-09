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
2. Add bundle config to Cargo.toml or Create a new bundle.toml.
3. Build the app by cargo.
4. Run the bundle command in the directory of the app. 
For example(used the config in the Cargo.toml): 
``` bash
cli bundle --verbose
```
By default, this will bundle the release binary. To bundle the debug binary instead, use the --debug flag.
You can also specify the path to the bundle config file by --config follow by the file path. If no path is specified, it will read config from the current package Cargo.toml file.

##### Bundle Config File Example
The bundle config file is in toml format. Here is an simple example:
``` toml
[bundle]
"productName" = "Counter"
"version" = "1.0.0"
"identifier" = "com.ribir.counter"
"shortDescription" = ""
"longDescription" = ""
"copyright" = "Copyright (c) You 2021. All rights reserved."
"icon" = ["../Logo.ico"]
"resources" = []  
"externalBin" = []
```
Note that this is just an example, and the actual configuration will depend on the specific requirements of your application. For more details, you can refer to the [`BundleConfig`] struct in the cli crate.

**Path Resolution**: Relative paths in the config file (such as `icon`, `resources`, `licenseFile`) are resolved relative to the config file's directory, not the current working directory. This makes it easier to organize your bundle configuration and assets together.
