FROM rust:1 as builder

WORKDIR /project/

COPY Cargo* ./
COPY src src

RUN cargo build --release

FROM gcr.io/distroless/cc-debian10

COPY --from=builder /project/target/release/zip-http-server /zip-http-server

USER nonroot

CMD ["/zip-http-server", "/root.zip"]
