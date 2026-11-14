# Imagem do SERVIDOR headless (P7). Build multi-stage: compila só o `server` (e o
# `shared`); o cliente/render não entram. Runtime slim — o servidor não tem renderer
# nem precisa de libs de áudio/gráficas (prova do desacoplamento, RFC 0001).

# ---- builder ----
FROM rust:1.100-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p server

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server /usr/local/bin/server

# Porta UDP do netcode (renet2). DATABASE_URL vem do ambiente (compose/k8s).
ENV RAGNADOFUS_PORT=5000
EXPOSE 5000/udp
ENTRYPOINT ["/usr/local/bin/server"]
