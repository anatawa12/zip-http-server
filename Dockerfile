# syntax=docker.io/docker/dockerfile:1.3.0
FROM rust:1 as builder

WORKDIR /project/

COPY Cargo* ./
COPY src src

RUN cargo build --release

FROM gcr.io/distroless/cc-debian10

COPY --from=builder /project/target/release/zip-http-server /zip-http-server
ARG TARGETARCH
ARG TARGETVARIANT
ADD --chmod=555 "https://api.anatawa12.com/short/tini-download?arch=${TARGETARCH}&variant=${TARGETVARIANT}" /tini

USER nonroot

CMD ["/tini", "--", "/zip-http-server", "/root.zip"]
