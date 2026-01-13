FROM mcr.microsoft.com/windows/servercore:ltsc2022

ARG RUST_STABLE_VERSION=1.92.0
ARG RUST_NIGHTLY_VERSION=2025-12-20
ARG DXC_RELEASE=v1.7.2308
ARG DXC_FILENAME=dxc_2023_08_14.zip
ARG WARP_VERSION=1.0.8

# Set working directory
WORKDIR C:/build

# Install Chocolatey
RUN powershell -Command \
    Set-ExecutionPolicy Bypass -Scope Process -Force; \
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; \
    iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# Install base tools using Chocolatey
RUN choco install -y git pkgconfiglite webp curl

# Install Visual Studio 2022 Build Tools
RUN powershell -Command \
    $ErrorActionPreference = 'Stop'; \
    Invoke-WebRequest -Uri https://aka.ms/vs/17/release/vs_buildtools.exe -OutFile vs_buildtools.exe; \
    Start-Process -FilePath .\vs_buildtools.exe -ArgumentList '--quiet', '--wait', '--norestart', '--noUpdateInstaller', \
        '--add', 'Microsoft.VisualStudio.Workload.VCTools', \
        '--add', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', \
        '--add', 'Microsoft.VisualStudio.Component.Windows11SDK.22621' -Wait; \
    Remove-Item vs_buildtools.exe; \
    if (Test-Path 'C:\ProgramData\Microsoft\VisualStudio\Packages') { \
        Remove-Item -Recurse -Force 'C:\ProgramData\Microsoft\VisualStudio\Packages' \
    }

# Install Rust
RUN powershell -Command \
    $ErrorActionPreference = 'Stop'; \
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
    Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe; \
    .\rustup-init.exe -y --default-toolchain stable --profile minimal; \
    Remove-Item rustup-init.exe;

# Configure Rust - Set PATH to include Cargo
RUN powershell -Command \
    [Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\Users\ContainerAdministrator\.cargo\bin', 'Machine')

# Install Rust toolchains and components
RUN powershell -Command \
    rustup default stable; \
    rustup install ${RUST_NIGHTLY_VERSION}; \
    rustup target add wasm32-unknown-unknown; \
    rustup component add rustfmt clippy llvm-tools-preview --toolchain ${RUST_NIGHTLY_VERSION}; \
    rustup component add llvm-tools-preview --toolchain stable

# Set up MSVC environment variables (Path, INCLUDE, LIB) for the system
RUN powershell -Command \
    $ErrorActionPreference = 'Stop'; \
    $vsPath = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools'; \
    $devShell = Join-Path $vsPath 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll'; \
    Import-Module $devShell; \
    Enter-VsDevShell -VsInstallPath $vsPath -SkipAutomaticLocation -Arch amd64; \
    [Environment]::SetEnvironmentVariable('Path', $env:Path, 'Machine'); \
    [Environment]::SetEnvironmentVariable('INCLUDE', $env:INCLUDE, 'Machine'); \
    [Environment]::SetEnvironmentVariable('LIB', $env:LIB, 'Machine'); \
    [Environment]::SetEnvironmentVariable('LIBPATH', $env:LIBPATH, 'Machine')

# Install Cargo tools
RUN powershell -Command \
    cargo install cargo-chef sccache cargo-llvm-cov cargo-nextest

# Install DirectX Shader Compiler (DXC)
RUN powershell -Command \
    Invoke-WebRequest -Uri https://github.com/microsoft/DirectXShaderCompiler/releases/download/${DXC_RELEASE}/${DXC_FILENAME} -OutFile dxc.zip; \
    Expand-Archive dxc.zip -DestinationPath dxc -Force; \
    [Environment]::SetEnvironmentVariable('Path', $env:Path + ';C:\build\dxc\bin\x64', 'Machine')

# Install WARP (Windows Advanced Rasterization Platform)
RUN powershell -Command \
    Invoke-WebRequest -Uri https://www.nuget.org/api/v2/package/Microsoft.Direct3D.WARP/${WARP_VERSION} -OutFile warp.zip; \
    Expand-Archive warp.zip -DestinationPath warp -Force; \
    Copy-Item warp\build\native\amd64\d3d10warp.dll C:\Windows\System32\ -Force

# Configure environment variables
RUN powershell -Command \
    [Environment]::SetEnvironmentVariable('RUSTC_WRAPPER', 'sccache', 'Machine'); \
    [Environment]::SetEnvironmentVariable('SCCACHE_DIR', 'C:\sccache', 'Machine'); \
    [Environment]::SetEnvironmentVariable('CARGO_INCREMENTAL', '0', 'Machine')

WORKDIR C:/app
LABEL org.opencontainers.image.source="https://github.com/RibirX/Ribir"
LABEL org.opencontainers.image.description="Ribir Windows Base Development Environment"
LABEL version="${RUST_STABLE_VERSION}-${RUST_NIGHTLY_VERSION}"
