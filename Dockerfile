FROM rust:1.94-slim AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
  pkg-config libssl-dev libsqlite3-dev \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY templates/ ./templates/
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates libsqlite3-0 wget \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/my-analytics .
COPY --from=builder /build/migrations/ ./migrations/
COPY --from=builder /build/templates/ ./templates/

RUN mkdir -p /app/data

EXPOSE 3000

ENV HOST=0.0.0.0
ENV PORT=3000
ENV DATABASE_URL=sqlite:///app/data/my_analytics.db
ENV RUST_LOG=my_analytics=debug,tower_http=debug

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD [ "sh", "-c", "wget -qO- http://localhost:3000/ > /dev/null 2>&1 || exit 1" ]

ENTRYPOINT ["./my-analytics"]
