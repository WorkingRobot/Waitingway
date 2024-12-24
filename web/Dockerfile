FROM clux/muslrust:stable AS chef
USER root
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1
RUN curl -sSL https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh && \
    chmod +x dotnet-install.sh && \
    ./dotnet-install.sh -Channel 9.0 -InstallDir /usr/share/dotnet && \
    ln -s /usr/share/dotnet/dotnet /usr/bin/dotnet
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine AS runtime
RUN addgroup -S waiting && adduser -S waiting -G waiting
USER waiting
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/waitingway-web .
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/connector/TemporalStasis.Connector connector/TemporalStasis.Connector
CMD ["./waitingway-web"]