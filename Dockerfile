# server
FROM busybox:stable-musl AS server

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/server /app/server

ENTRYPOINT ["/app/server"]


# agent
FROM alpine:3.24 AS agent


RUN apk add --no-cache bird wireguard-tools bind-tools

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/agent /app/agent

# get tcping
RUN wget https://github.com/pouriyajamshidi/tcping/releases/download/v2.8.0/tcping-linux-amd64-static.tar.gz -O tcping.tar.gz && \
    tar -xzf tcping.tar.gz -C /usr/bin && \
    rm tcping.tar.gz

ENTRYPOINT ["/app/agent"]


# tgbot
FROM debian:trixie-slim AS tgbot

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    fonts-liberation \
    wget \
    && rm -rf /var/lib/apt/lists/*

## get custom ttf for CJK
RUN wget "https://raw.githubusercontent.com/ZaneL1u/TTF-Min/main/dist/ttf-%E6%80%9D%E6%BA%90%E9%BB%91%E4%BD%93/SourceHanSansSC-Regular.ttf" -O /usr/share/fonts/SourceHanSansSC-Regular_ttfmin.ttf

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/tgbot /app/tgbot

# headless_chrome won't recognize chromium-headless-shell
ENV CHROME=/usr/bin/chromium-headless-shell

ENTRYPOINT ["/app/tgbot"]
