# Zip Http Server

[![a12 maintenance: Slowly](https://anatawa12.com/short.php?q=a12-slowly-svg)](https://anatawa12.com/short.php?q=a12-slowly-doc)
[![Crates.io (latest)](https://img.shields.io/crates/dv/zip-http-server)](https://crates.io/crates/zip-http-server)
[![github packages download](https://img.shields.io/badge/packages-download-green?logo=github)](https://github.com/anatawa12/zip-http-server/pkgs/container/zip-http-server)

The http server exports contents in zip file.

## Stability warning

All APIs including command line interface, docker container are not yet stable.
They can be changed in the feature. 

## How to use

### via docker

This server is arrival on [ghcr.io, github packages container registry][ghcr.io].

Currently, custom port, address, location of zip is not yet supported but will be supported.

```shell
docker run -p 80:80 -v '/path/to/zip/file:/root.zip' ghcr.io/anatawa12/zip-http-server 
```

[ghcr.io]: https://ghcr.io/

### install via cargo

This server is also arrival on [crates.io]. 
You can install via [cargo] command.
For more options, please see `--help`.

```shell
cargo install zip-http-server
zip-http-server /path/to/zip/file
```

[cargo]: https://github.com/rust-lang/cargo
[crates.io]: https://crates.io/
