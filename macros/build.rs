//! Build script for ribir_macros
//!
//! This script checks for the necessary build environment required by the
//! webp-animation crate (which depends on libwebp via C bindings).

use std::process::Command;

fn main() {
  // Check if C compiler is available
  if !has_c_compiler() {
    print_c_compiler_instructions();
    panic!("C compiler not found. Please install the required build tools.");
  }

  // Check if libwebp is available
  if !has_libwebp() {
    print_libwebp_instructions();
    panic!("libwebp not found. Please install the required library.");
  }

  println!("cargo:rerun-if-changed=build.rs");
}

/// Check if a C compiler is available on the system.
fn has_c_compiler() -> bool {
  let compilers = if cfg!(target_os = "windows") {
    vec!["cl.exe", "clang.exe", "gcc.exe"]
  } else {
    vec!["cc", "clang", "gcc"]
  };

  for compiler in compilers {
    if Command::new(compiler)
      .arg("--version")
      .output()
      .map(|o| o.status.success())
      .unwrap_or(false)
    {
      return true;
    }
  }

  false
}

/// Check if libwebp is available on the system and emit search paths if needed.
fn has_libwebp() -> bool {
  // Try pkg-config first (Linux/macOS) - it also handles setting library paths
  if Command::new("pkg-config")
    .args(["--exists", "libwebp"])
    .output()
    .map(|o| o.status.success())
    .unwrap_or(false)
  {
    // pkg-config found it, emit the library path
    if let Some(output) = Command::new("pkg-config")
      .args(["--libs-only-L", "libwebp"])
      .output()
      .map(|o| if o.status.success() { Some(o) } else { None })
      .ok()
      .flatten()
    {
      let libs = String::from_utf8_lossy(&output.stdout);
      for lib in libs.split_whitespace() {
        if let Some(path) = lib.strip_prefix("-L") {
          println!("cargo:rustc-link-search=native={}", path);
        }
      }
    }
    return true;
  }

  // Check common library paths on macOS (Homebrew)
  #[cfg(target_os = "macos")]
  {
    let brew_lib_dirs = [
      "/opt/homebrew/opt/webp/lib", // Apple Silicon
      "/usr/local/opt/webp/lib",    // Intel
      "/opt/homebrew/lib",          // Apple Silicon fallback
      "/usr/local/lib",             // Intel fallback
    ];

    for lib_dir in brew_lib_dirs {
      let lib_path = format!("{}/libwebp.dylib", lib_dir);
      if std::path::Path::new(&lib_path).exists() {
        // Found libwebp, emit search path for linker
        println!("cargo:rustc-link-search=native={}", lib_dir);
        // Also emit include path if it exists
        let include_dir = lib_dir.replace("/lib", "/include");
        if std::path::Path::new(&include_dir).exists() {
          println!("cargo:include={}", include_dir);
        }
        return true;
      }
    }
  }

  // Check common library paths on Linux
  #[cfg(target_os = "linux")]
  {
    let linux_paths = [
      "/usr/lib/libwebp.so",
      "/usr/lib/x86_64-linux-gnu/libwebp.so",
      "/usr/lib/aarch64-linux-gnu/libwebp.so",
      "/usr/local/lib/libwebp.so",
    ];
    for path in linux_paths {
      if std::path::Path::new(path).exists() {
        // Most Linux paths are already in the default search path
        return true;
      }
    }
  }

  // Windows: check if webp.lib exists in common paths
  #[cfg(target_os = "windows")]
  {
    // On Windows, vcpkg or manual installation might put it in various places
    // The libwebp-sys crate can build from source, so we're more lenient here
    true
  }
  #[cfg(not(target_os = "windows"))]
  {
    false
  }
}

/// Print C compiler installation instructions.
fn print_c_compiler_instructions() {
  eprintln!();
  eprintln!("╔══════════════════════════════════════════════════════════════════════════════╗");
  eprintln!("║                     Missing C Compiler                                       ║");
  eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
  eprintln!("║ The `ribir_macros` crate requires a C compiler to build the `webp-animation` ║");
  eprintln!("║ dependency, which converts images to WebP format at compile time.            ║");
  eprintln!("╚══════════════════════════════════════════════════════════════════════════════╝");
  eprintln!();

  #[cfg(target_os = "macos")]
  {
    eprintln!("🍎 macOS: Install Xcode Command Line Tools:");
    eprintln!();
    eprintln!("    xcode-select --install");
    eprintln!();
  }

  #[cfg(target_os = "linux")]
  {
    eprintln!("🐧 Linux: Install build essentials:");
    eprintln!();
    eprintln!("  Ubuntu/Debian:");
    eprintln!("    sudo apt update && sudo apt install build-essential");
    eprintln!();
    eprintln!("  Fedora/RHEL:");
    eprintln!("    sudo dnf groupinstall 'Development Tools'");
    eprintln!();
    eprintln!("  Arch Linux:");
    eprintln!("    sudo pacman -S base-devel");
    eprintln!();
  }

  #[cfg(target_os = "windows")]
  {
    eprintln!("🪟 Windows: Install Visual Studio Build Tools:");
    eprintln!();
    eprintln!("  1. Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/");
    eprintln!("  2. Run the installer and select 'Desktop development with C++'");
    eprintln!();
  }

  eprintln!("After installation, please restart your terminal and try building again.");
  eprintln!();
}

/// Print libwebp installation instructions.
fn print_libwebp_instructions() {
  eprintln!();
  eprintln!("╔══════════════════════════════════════════════════════════════════════════════╗");
  eprintln!("║                     Missing libwebp Library                                  ║");
  eprintln!("╠══════════════════════════════════════════════════════════════════════════════╣");
  eprintln!("║ The `ribir_macros` crate requires libwebp to encode images to WebP format.   ║");
  eprintln!("║ Please install it using your system's package manager.                       ║");
  eprintln!("╚══════════════════════════════════════════════════════════════════════════════╝");
  eprintln!();

  #[cfg(target_os = "macos")]
  {
    eprintln!("🍎 macOS: Install via Homebrew:");
    eprintln!();
    eprintln!("    brew install webp");
    eprintln!();
  }

  #[cfg(target_os = "linux")]
  {
    eprintln!("🐧 Linux: Install via package manager:");
    eprintln!();
    eprintln!("  Ubuntu/Debian:");
    eprintln!("    sudo apt update && sudo apt install libwebp-dev");
    eprintln!();
    eprintln!("  Fedora/RHEL:");
    eprintln!("    sudo dnf install libwebp-devel");
    eprintln!();
    eprintln!("  Arch Linux:");
    eprintln!("    sudo pacman -S libwebp");
    eprintln!();
  }

  #[cfg(target_os = "windows")]
  {
    eprintln!("🪟 Windows: Install via vcpkg:");
    eprintln!();
    eprintln!("    vcpkg install libwebp");
    eprintln!();
    eprintln!("  Or download pre-built binaries from:");
    eprintln!("    https://developers.google.com/speed/webp/download");
    eprintln!();
  }

  #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
  {
    eprintln!("Please install libwebp for your platform.");
    eprintln!();
  }

  eprintln!("After installation, please restart your terminal and try building again.");
  eprintln!();
}
