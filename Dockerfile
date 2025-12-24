FROM rust:alpine AS build

ADD . /app

WORKDIR /app

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig \
    ca-certificates

RUN cargo build --all-features --release -p kintsu-registry && \
    mv ./target/release/kintsu-registry /app/kintsu-registry

FROM alpine:latest

RUN apk --no-cache add ca-certificates

RUN addgroup -S kintsu \
    && adduser -S -G kintsu kintsu \
    && mkdir -p /apps/kintsu-registry \
    && chown -R kintsu:kintsu /apps/kintsu-registry

COPY --from=build /app/kintsu-registry /usr/local/bin/kintsu-registry

USER kintsu

WORKDIR /apps/kintsu-registry

VOLUME [ "/apps/kintsu-registry/config" ]
EXPOSE 8000

CMD ["kintsu-registry", "-d", "/apps/kintsu-registry/config"]
