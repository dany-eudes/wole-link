# Rust toolchain for local builds; nothing to install on the host.
# Linux: static musl binaries. Windows: built with MinGW for testing in a VM
# (release builds for Windows come from CI on the MSVC toolchain).
FROM rust:1-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends musl-tools gcc-mingw-w64-x86-64 \
 && rm -rf /var/lib/apt/lists/* \
 && rustup target add x86_64-unknown-linux-musl x86_64-pc-windows-gnu \
 # Cargo registry cache (a named volume) must be writable by the host uid.
 && mkdir -p /cargo && chmod 1777 /cargo
