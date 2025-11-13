FROM rust:1.91.1-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY shared ./shared
COPY launcher ./launcher
COPY instance_builder ./instance_builder
COPY Cargo.toml ./Cargo.toml
COPY Cargo.lock ./Cargo.lock

WORKDIR /build/instance_builder
RUN cargo build --release -p instance_builder


FROM python:3.14-slim

WORKDIR /backend

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY backend/pyproject.toml ./pyproject.toml
RUN curl -LsSf https://astral.sh/uv/install.sh | sh && ln -s /root/.local/bin/uv /usr/local/bin/uv \
    && uv sync --no-dev

COPY backend/app/ ./app

COPY --from=builder /build/target/release/instance_builder /usr/local/bin/instance_builder

EXPOSE 8000

CMD ["uv", "run", "-m", "app.main"]
