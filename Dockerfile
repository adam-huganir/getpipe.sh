FROM rust:alpine3.23 AS build

RUN apk update \
  && apk add --no-cache musl-dev git curl ca-certificates \
  && rustup target add x86_64-unknown-linux-musl

WORKDIR /app

# Cache dependency compilation. Copy manifests first; build with stub entry points;
# then overlay real source so only changed application code triggers a full rebuild.
COPY Cargo.toml Cargo.lock ./
COPY xtask/Cargo.toml xtask/
RUN mkdir -p src xtask/src \
  && echo 'fn main() {}' > src/main.rs \
  && echo 'fn main() {}' > xtask/src/main.rs \
  && cargo build --release --target x86_64-unknown-linux-musl \
  && rm -rf src xtask/src

COPY . .
RUN touch src/main.rs xtask/src/main.rs \
  && cargo build --release --target x86_64-unknown-linux-musl

RUN printf "user:x:1000:1000::/nonexistent:/sbin/nologin\n" > /tmp/passwd \
  && printf "user:x:1000:\n" > /tmp/group

FROM scratch AS runtime

COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/getpipe /getpipe

ENTRYPOINT ["/getpipe"]
