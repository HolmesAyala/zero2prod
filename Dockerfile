# Version 2

FROM lukemathwalker/cargo-chef:latest-rust-1.93-slim AS chef_stage
WORKDIR /app
# RUN apt update && apt install lld clang -y

FROM chef_stage as planner_stage
copy . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef_stage as builder_stage
COPY --from=planner_stage /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin zero2prod

FROM debian:bookworm-slim AS runtime_stage
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder_stage /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]

# Version 1

# Build stage
# FROM rust:1.93 AS builder_stage
# WORKDIR /app
# # RUN apt-get update && apt-get install lld clang -y
# COPY . .
# ENV SQLX_OFFLINE true
# RUN cargo build --release

# # Runtime stage
# FROM debian:bookworm-slim AS runtime_stage
# WORKDIR /app
# RUN apt-get update -y \
#     && apt-get install -y --no-install-recommends openssl ca-certificates \
#     # Clean up
#     && apt-get autoremove -y \
#     && apt-get clean -y \
#     && rm -rf /var/lib/apt/lists/*
# COPY --from=builder_stage /app/target/release/zero2prod zero2prod
# COPY configuration configuration
# ENV APP_ENVIRONMENT production
# ENTRYPOINT ["./zero2prod"]
