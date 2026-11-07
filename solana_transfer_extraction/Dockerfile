FROM ghcr.io/streamingfast/substreams:v1.26.0

USER root

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:/app:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fSL \
    "https://github.com/bufbuild/buf/releases/download/v1.75.0/buf-Linux-x86_64" \
    -o /usr/local/bin/buf \
    && chmod +x /usr/local/bin/buf

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable

RUN rustup target add wasm32-unknown-unknown

RUN buf --version && rustc --version && cargo --version

WORKDIR /workspace