use std::{
  path::{Path, PathBuf},
  str::FromStr,
  time::Duration,
};

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser};
use ignore::gitignore::Gitignore;
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer, notify::*};

use crate::{
  CliCommand,
  program_check::{Program, check_all_programs},
};

const WATCH_DEBOUNCE_GAP: Duration = Duration::from_secs(2);
const DEBUG_BRIDGE_INJECT_START: &str = "<!-- RIBIR_DEBUG_BRIDGE_START -->";
const DEBUG_BRIDGE_INJECT_END: &str = "<!-- RIBIR_DEBUG_BRIDGE_END -->";

/// Middleware to add COEP/COOP headers required for WASM SharedArrayBuffer.
async fn add_cors_headers(
  req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next,
) -> axum::http::Response<axum::body::Body> {
  use axum::http::{HeaderName, HeaderValue};
  let mut res = next.run(req).await;
  res.headers_mut().insert(
    HeaderName::from_static("cross-origin-embedder-policy"),
    HeaderValue::from_static("require-corp"),
  );
  res.headers_mut().insert(
    HeaderName::from_static("cross-origin-opener-policy"),
    HeaderValue::from_static("same-origin"),
  );
  res
}

pub fn run_wasm() -> Box<dyn CliCommand> { Box::new(RunWasm {}) }

struct RunWasm {}
#[derive(Parser, Debug, Clone)]
#[command(name = "run-wasm")]
/// run as web wasm
struct Wasm {
  /// Package of which to run
  #[arg(short, long)]
  package: String,

  /// Name of output, default to web_wasm
  #[arg(short, long)]
  name: Option<String>,

  /// Direction path to output, default to target/wasm
  #[arg(short, long)]
  out_dir: Option<PathBuf>,

  /// Build release, default build to debug
  #[arg(short, long)]
  release: bool,

  /// Just build the wasm files, don't serve them
  #[arg(long, name = "no-server")]
  no_server: bool,

  /// Template files need to copy to Output dir
  #[arg(short, long)]
  template: Option<PathBuf>,

  /// HTTP server host for serving wasm files
  #[arg(long, default_value = "127.0.0.1")]
  host: String,

  /// HTTP server port for serving wasm files
  #[arg(long, default_value_t = 8000)]
  port: u16,

  /// Enable debug feature and start bridge server for wasm debugging
  #[arg(long)]
  debug: bool,

  /// Bridge server host (only used with --debug)
  #[arg(long, default_value = "127.0.0.1")]
  bridge_host: String,

  /// Bridge server port (only used with --debug)
  #[arg(long, default_value_t = 2333)]
  bridge_port: u16,
}

impl Wasm {
  fn root_path(&self) -> Result<PathBuf> {
    let root = PathBuf::from_str(env!("CARGO_WORKSPACE_DIR"))?;
    Ok(root)
  }

  fn out_dir(&self) -> PathBuf {
    let out_dir = self
      .out_dir
      .clone()
      .unwrap_or(PathBuf::from("./target/wasm"));

    if out_dir.is_relative() {
      let root_path = PathBuf::from_str(env!("CARGO_WORKSPACE_DIR")).unwrap();
      root_path.clone().join(&out_dir)
    } else {
      out_dir.clone()
    }
  }

  fn output_name(&self) -> String {
    self
      .name
      .clone()
      .unwrap_or("web_wasm".to_string())
  }

  fn auto_rebuild(&self) -> Debouncer<RecommendedWatcher> {
    let root_path = self.root_path().unwrap();
    let ignore_file = root_path.join(".gitignore");

    // Build gitignore matcher using the `ignore` crate
    // Gitignore::new() parses the .gitignore file and returns a matcher
    // that resolves patterns relative to the gitignore file's parent directory.
    let (gitignore, err) = Gitignore::new(&ignore_file);
    if let Some(e) = err {
      eprintln!("Warning: some gitignore patterns failed to parse: {}", e);
    }

    let this = self.clone();
    let mut debouncer =
      new_debouncer(WATCH_DEBOUNCE_GAP, move |res: DebounceEventResult| match res {
        Ok(events) => {
          let need_rebuild = events.iter().any(|e| {
            // Use matched_path_or_any_parents because `/target/` pattern only
            // matches the directory itself, not files within it. This method
            // walks up parent directories to check if any ancestor is ignored.
            !gitignore
              .matched_path_or_any_parents(&e.path, false)
              .is_ignore()
          });
          if need_rebuild {
            let _ = this.wasm_build();
          }
        }
        Err(e) => eprintln!("Watch error: {:?}", e),
      })
      .unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debouncer
      .watcher()
      .watch(&root_path, RecursiveMode::Recursive)
      .unwrap();
    debouncer
  }

  fn wasm_build(&self) -> Result<()> {
    let shell = xshell::Shell::new()?;
    let root_path = self.root_path()?;
    let output = self.out_dir();
    let package = self.package.clone();

    shell.set_var("RUSTFLAGS", "--cfg getrandom_backend=\"wasm_js\"");
    let build_cmd =
      xshell::cmd!(shell, "cargo build -p {package} --lib --target wasm32-unknown-unknown");
    let build_cmd = if self.release { build_cmd.arg("--release") } else { build_cmd };
    let build_cmd = if self.debug { build_cmd.arg("--features").arg("debug") } else { build_cmd };
    build_cmd.quiet().run()?;

    shell.change_dir(env!("CARGO_WORKSPACE_DIR"));
    let target_path = if self.release {
      "target/wasm32-unknown-unknown/release"
    } else {
      "target/wasm32-unknown-unknown/debug"
    };

    let package = self.package.clone();
    let name = self.output_name();
    xshell::cmd!(
      shell,
      "wasm-bindgen {target_path}/{package}.wasm --target web
    --no-typescript --out-dir {output} --out-name {name}"
    )
    .quiet()
    .run()?;

    // Copy user-specified template if provided
    if let Some(mut path) = self.template.clone() {
      if path.is_relative() {
        path = root_path.clone().join(path);
      }
      if path.is_dir() {
        fs_extra::dir::copy(
          &path,
          &output,
          &fs_extra::dir::CopyOptions::new()
            .overwrite(true)
            .content_only(true),
        )?;
      } else {
        let file_name = output.clone().join(path.file_name().unwrap());
        fs_extra::file::copy(
          &path,
          file_name,
          &fs_extra::file::CopyOptions::new().overwrite(true),
        )?;
      }
    }

    // Generate or update index.html
    self.generate_html(&output)?;

    Ok(())
  }

  fn server(&self) -> Result<()> {
    let _watcher = self.auto_rebuild();

    let out_dir = self.out_dir();
    let port = self.port;
    let host = self.host.clone();

    // Serve WASM files using axum + tower_http
    let handle = std::thread::Builder::new().spawn(move || {
      let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

      rt.block_on(async {
        use axum::{Router, middleware};
        use tower_http::services::{ServeDir, ServeFile};

        let out_dir = out_dir.clone();
        let index_html = out_dir.join("index.html");

        // Serve static files from output directory
        let serve_dir = ServeDir::new(&out_dir).not_found_service(ServeFile::new(index_html));

        let app = Router::new()
          .fallback_service(serve_dir)
          .layer(middleware::from_fn(add_cors_headers));

        let addr = format!("{}:{}", host, port);
        let listener = match tokio::net::TcpListener::bind(&addr).await {
          Ok(l) => l,
          Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            return;
          }
        };

        eprintln!("Serving WASM files at http://{}", addr);
        if let Err(e) = axum::serve(listener, app).await {
          eprintln!("Server error: {}", e);
        }
      });
    })?;

    handle.join().unwrap();
    Ok(())
  }

  fn strip_debug_injection(&self, html: String) -> String {
    if let Some(start) = html.find(DEBUG_BRIDGE_INJECT_START) {
      if let Some(end_rel) = html[start..].find(DEBUG_BRIDGE_INJECT_END) {
        let end = start + end_rel + DEBUG_BRIDGE_INJECT_END.len();
        let mut stripped = String::with_capacity(html.len());
        stripped.push_str(&html[..start]);
        stripped.push_str(&html[end..]);
        return stripped;
      }
    }
    html
  }

  fn generate_html(&self, output: &Path) -> Result<()> {
    let html_path = output.join("index.html");

    // Get bridge URLs for debug mode
    let bridge_urls = if self.debug {
      let http_url = std::env::var("RIBIR_DEBUG_URL")
        .ok()
        .unwrap_or_else(|| format!("http://{}:{}", self.bridge_host, self.bridge_port));
      let ws_url = if let Some(host) = http_url.strip_prefix("https://") {
        format!("wss://{}/ws", host)
      } else if let Some(host) = http_url.strip_prefix("http://") {
        format!("ws://{}/ws", host)
      } else {
        format!(
          "ws://{}/ws",
          http_url
            .trim_start_matches("ws://")
            .trim_start_matches("wss://")
        )
      };
      Some((http_url, ws_url))
    } else {
      None
    };

    // Check if user template already generated an index.html
    let html_content = if html_path.exists() {
      std::fs::read_to_string(&html_path)?
    } else {
      // Use built-in template
      include_str!("../template/index.html").to_string()
    };
    let html_content = self.strip_debug_injection(html_content);

    // In debug mode, inject bridge script
    let final_html = if self.debug {
      let bridge_http_url = bridge_urls
        .map(|(http, _ws)| http)
        .unwrap_or_else(|| format!("http://{}:{}", self.bridge_host, self.bridge_port));

      let injected_script = format!(
        r#"{DEBUG_BRIDGE_INJECT_START}
<script>
// Auto-inject debug server URL for debug mode
const url = new URL(window.location);
if (!url.searchParams.has('ribir_debug_server')) {{
  url.searchParams.set('ribir_debug_server', '{bridge_http_url}');
  window.history.replaceState(null, '', url);
}}
console.log('Ribir debug server:', '{bridge_http_url}');
</script>
{DEBUG_BRIDGE_INJECT_END}"#
      );

      if html_content.contains("</body>") {
        html_content.replace("</body>", &format!("{}\n</body>", injected_script))
      } else {
        html_content + &injected_script
      }
    } else {
      html_content
    };

    std::fs::write(&html_path, final_html)?;
    Ok(())
  }
}

impl CliCommand for RunWasm {
  fn name(&self) -> &str { "run-wasm" }

  fn command(&self) -> clap::Command { Wasm::command() }

  fn exec(&self, args: &clap::ArgMatches) -> Result<()> {
    let args = Wasm::from_arg_matches(args)?;

    let dependencies =
      vec![Program { crate_name: "wasm-bindgen-cli", binary_name: "wasm-bindgen" }];
    check_all_programs(&dependencies)?;

    args.wasm_build()?;
    if !args.no_server {
      args.server()?;
    }

    Ok(())
  }
}
