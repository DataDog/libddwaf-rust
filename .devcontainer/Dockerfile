FROM rust:latest
RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt \
    --mount=type=tmpfs,target=/tmp \
  apt-get update && \
  apt-get install -y libclang-dev gdb
