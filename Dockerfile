# server
FROM busybox:stable-musl AS server

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/server /app/server

ENTRYPOINT ["/app/server"]


# agent
FROM busybox:stable-musl AS agent

WORKDIR /app

COPY target/x86_64-unknown-linux-musl/release/agent /app/agent

ENTRYPOINT ["/app/agent"]


# todo: add tools like tcping