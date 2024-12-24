FROM rust:alpine AS chef
USER root
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1
RUN apk add musl-dev dotnet9-sdk
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM alpine AS runtime
RUN addgroup -S waiting && adduser -S waiting -G waiting
USER waiting
WORKDIR /app
COPY --from=builder /app/target/release/waitingway-web waitingway-web
COPY --from=builder /app/target/release/TemporalStasis.Connector TemporalStasis.Connector
CMD ["./waitingway-web"]