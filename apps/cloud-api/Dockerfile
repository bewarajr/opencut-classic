FROM rust:1.89-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY apps/cloud-api ./apps/cloud-api
COPY rust ./rust
COPY apps/desktop ./apps/desktop
RUN cargo build --release -p opencut-cloud-api

FROM alpine:3.22
RUN apk add --no-cache ca-certificates libgcc
RUN addgroup -S opencut && adduser -S -G opencut opencut
WORKDIR /app
COPY --from=builder /build/target/release/opencut-cloud-api /usr/local/bin/opencut-cloud-api
RUN mkdir -p /data/media && chown -R opencut:opencut /data
USER opencut
ENV PORT=8080
ENV MEDIA_ROOT=/data/media
EXPOSE 8080
CMD ["opencut-cloud-api"]
