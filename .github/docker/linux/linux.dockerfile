# syntax=docker/dockerfile:1.4
FROM ubuntu:22.04

ARG RUST_STABLE_VERSION=1.92.0
ARG RUST_NIGHTLY_VERSION=2025-12-20
ARG VULKAN_SDK_VERSION=1.3.268
ARG MESA_VERSION=23.3.1
ARG CI_BINARY_BUILD=build18

# Avoid prompts
ENV DEBIAN_FRONTEND=noninteractive

# Install base tools and dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
    gnupg \
    gcc \
    g++ \
    make \
    pkg-config \
    libasound2-dev \
    libwebp-dev \
    libssl-dev \
    libfontconfig1-dev \
    libfreetype6-dev \
    libx11-dev \
    libxcursor-dev \
    libxi-dev \
    libxrandr-dev \
    libxinerama-dev \
    libxcomposite-dev \
    libxdamage-dev \
    libwayland-dev \
    libxkbcommon-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Vulkan SDK
RUN wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | apt-key add - \
    && wget -qO /etc/apt/sources.list.d/lunarg-vulkan-${VULKAN_SDK_VERSION}-jammy.list \
    https://packages.lunarg.com/vulkan/${VULKAN_SDK_VERSION}/lunarg-vulkan-${VULKAN_SDK_VERSION}-jammy.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    vulkan-sdk \
    mesa-vulkan-drivers \
    libgl1-mesa-dev \
    && rm -rf /var/lib/apt/lists/*

# Download and extract Mesa build from gfx-rs/ci-build
RUN curl -L --retry 5 \
    https://github.com/gfx-rs/ci-build/releases/download/${CI_BINARY_BUILD}/mesa-${MESA_VERSION}-linux-x86_64.tar.xz \
    -o mesa.tar.xz \
    && mkdir -p /app/mesa \
    && tar xpf mesa.tar.xz -C /app/mesa \
    && rm mesa.tar.xz

# Configure ICD for Vulkan
RUN cat <<EOF > /opt/icd.json
{
  "ICD": {
      "api_version": "1.1.255",
      "library_path": "/app/mesa/lib/x86_64-linux-gnu/libvulkan_lvp.so"
  },
  "file_format_version": "1.0.0"
}
EOF

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain none \
    --profile minimal \
    && . "$HOME/.cargo/env" \
    && rustup toolchain install ${RUST_STABLE_VERSION} \
    && rustup toolchain install ${RUST_NIGHTLY_VERSION} \
    && rustup toolchain link stable ${RUST_STABLE_VERSION} \
    && rustup toolchain link nightly ${RUST_NIGHTLY_VERSION} \
    && rustup default stable \
    && rustup target add wasm32-unknown-unknown \
    && rustup component add rustfmt clippy llvm-tools-preview --toolchain nightly \
    && rustup component add llvm-tools-preview --toolchain stable

# Configure environment
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Cargo tools
RUN cargo install cargo-chef sccache cargo-llvm-cov cargo-nextest cargo-workspaces

# Configure sccache
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache
ENV SCCACHE_CACHE_SIZE=10G
ENV CARGO_INCREMENTAL=0
RUN mkdir -p /sccache && chmod 777 /sccache

# Configure environment variables for GPU testing
ENV VK_DRIVER_FILES=/opt/icd.json
ENV LD_LIBRARY_PATH=/app/mesa/lib/x86_64-linux-gnu/:${LD_LIBRARY_PATH}
ENV LIBGL_DRIVERS_PATH=/app/mesa/lib/x86_64-linux-gnu/dri
ENV MESA_LOADER_DRIVER_OVERRIDE=zink

# Pre-warm sccache with dependencies from master
ARG PREWARM_CACHE=false
RUN if [ "$PREWARM_CACHE" = "true" ]; then \
    git clone --depth 1 https://github.com/RibirX/Ribir.git /tmp/ribir && \
    cd /tmp/ribir && \
    cargo test --workspace --all-features --no-run && \
    cd / && \
    rm -rf /tmp/ribir; \
    fi

RUN git clone --depth 1 https://github.com/RibirX/Ribir.git /tmp/ribir-ci && \
    cd /tmp/ribir-ci && \
    cargo +nightly ci config-nightly && \
    cd / && \
    rm -rf /tmp/ribir-ci

WORKDIR /app
LABEL org.opencontainers.image.source="https://github.com/RibirX/Ribir"
LABEL org.opencontainers.image.description="Ribir Linux Development & Testing Environment"
LABEL version="${RUST_STABLE_VERSION}-${RUST_NIGHTLY_VERSION}"
LABEL gpu-support=true
