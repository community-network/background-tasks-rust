
FROM rust:1.82 as builder
WORKDIR /usr/src/myapp
COPY . .
ARG github_token 

RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install protobuf-compiler libprotobuf-dev -y && apt-get clean
RUN git config --global credential.helper store && echo "https://zefanjajobse:${github_token}@github.com" > ~/.git-credentials && cargo install --path .











FROM debian:bookworm-slim

# Install Opn SSL libs

RUN apt-get update -y && apt-get install pkg-config libssl-dev -y

EXPOSE 3030

HEALTHCHECK --interval=5m --timeout=3s --start-period=5s \
  CMD curl -f http://127.0.0.1:3030/ || exit 1

# RUN echo "/usr/local/openssl/lib" >> /etc/ld.so.conf
# RUN ldconfig
COPY server.pem .
COPY --from=builder /usr/local/cargo/bin/background-tasks-rust /usr/local/bin/background-tasks-rust
RUN apt-get update && apt-get upgrade -y && apt-get install --assume-yes curl protobuf-compiler libprotobuf-dev && apt-get clean
CMD background-tasks-rust
