# server
FROM busybox:stable-musl AS server

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/server /app/server

ENTRYPOINT ["/app/server"]


# agent
FROM alpine:3.23 AS agent

RUN apk update && apk add --no-cache bird

RUN apk add --no-cache bird wireguard-tools

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/agent /app/agent

# get tcping
RUN wget https://github.com/pouriyajamshidi/tcping/releases/download/v2.8.0/tcping-linux-amd64-static.tar.gz -O tcping.tar.gz && \
    tar -xzf tcping.tar.gz -C /usr/bin && \
    rm tcping.tar.gz

ENTRYPOINT ["/app/agent"]
