FROM rust:1.69.0 AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/signaling-server

FROM chef AS planner
# Copy the folders seperatly to improve layer caching,
# folders go from least to most likely to change.
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /usr/src/signaling-server/recipe.json recipe.json
# TODO(perf): bring back once we end quick iteration phase
# RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the folders seperatly to improve layer caching,
# folders go from least to most likely to change.
COPY . .

# This is the actual application build.
RUN cargo build --release

################
##### Runtime
FROM debian:bullseye-slim AS runtime

# Install CA certificates
RUN apt-get update && apt-get upgrade -y && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy application binary from builder image
COPY --from=builder /usr/src/signaling-server/target/release/wasm-peers-signaling-server /usr/local/bin/

EXPOSE 9001

# Run the application
CMD /usr/local/bin/wasm-peers-signaling-server
