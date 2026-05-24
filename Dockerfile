FROM rust:alpine3.23 AS build

RUN apk update \
  && apk add --no-cache musl-dev git curl ca-certificates \
  && rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

RUN printf "user:x:1000:1000::/nonexistent:/sbin/nologin\n" > /tmp/passwd \
  && printf "user:x:1000:\n" > /tmp/group

FROM scratch AS runtime

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/termlib-server /termlib-server

ENTRYPOINT ["/termlib-server"]
