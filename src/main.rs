use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Response};
use simple_logger::SimpleLogger;
use std::convert::Infallible;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Arc;
use zip::result::ZipError;
use zip::ZipArchive;
use std::net::IpAddr;
use clap::Clap;

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opts {
    zip_file: PathBuf,
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "0.0.0.0")]
    address: IpAddr,
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "80")]
    port: u16,
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
    let options: Opts = Opts::parse();

    log::info!("starting server...");

    let zip_file: PathBuf = options.zip_file;
    log::info!("using zip {}", &zip_file.display());
    let zip_file: Arc<PathBuf> = Arc::new(zip_file).into();

    // And a MakeService to handle each connection...
    let make_svc = make_service_fn(move |_| {
        let zip_file = zip_file.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let zip_file = zip_file.clone();
                async move {
                    let res = match handle(zip_file, req.uri().path()).await {
                        Ok(vec) => Response::builder().body(Body::from(vec)).unwrap(),
                        Err(HandleError::IO(err)) => {
                            log::error!("io error {}", err);
                            internal_server_error()
                        }
                        Err(HandleError::Invalid(str)) => {
                            log::error!("invalid archive {}", str);
                            internal_server_error()
                        }
                        Err(HandleError::NotFound) => {
                            log::error!("not found: {}", req.uri().path());
                            not_found()
                        }
                    };
                    Ok::<_, Infallible>(res)
                }
            }))
        }
    });

    let server = hyper::Server::bind(&(options.address, options.port).into()).serve(make_svc);

    log::info!("server started!");
    // Run forever-ish...
    if let Err(err) = server.await {
        log::error!("server error: {}", err);
    }
}

fn internal_server_error() -> Response<Body> {
    Response::builder()
        .status(500)
        .header("Content-Type", "text/plain")
        .body(Body::from("internal server error"))
        .unwrap()
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Body::from("NOT FOUND"))
        .unwrap()
}

enum HandleError {
    IO(std::io::Error),
    Invalid(&'static str),
    NotFound,
}

impl From<std::io::Error> for HandleError {
    fn from(err: std::io::Error) -> Self {
        HandleError::IO(err)
    }
}

impl From<ZipError> for HandleError {
    fn from(err: ZipError) -> Self {
        match err {
            ZipError::FileNotFound => HandleError::NotFound,
            ZipError::InvalidArchive(err) => HandleError::Invalid(err),
            ZipError::UnsupportedArchive(err) => HandleError::Invalid(err),
            ZipError::Io(err) => HandleError::IO(err),
        }
    }
}

async fn handle(zip_file: Arc<PathBuf>, name: &str) -> Result<Vec<u8>, HandleError> {
    let name = name.to_owned();
    match tokio::task::spawn_blocking(move || handle_blocking(zip_file, name)).await {
        Ok(res) => res,
        Err(err) => std::panic::resume_unwind(err.into_panic()),
    }
}

fn handle_blocking(zip_file: Arc<PathBuf>, mut name: String) -> Result<Vec<u8>, HandleError> {
    if name.ends_with('/') {
        name = format!("{}index.html", name);
    }
    let name = name.strip_prefix('/').unwrap_or(&name);
    let zip_file = File::open(&*zip_file)?;
    let zip_file = BufReader::new(zip_file);
    let mut zip_file = ZipArchive::new(zip_file)?;

    log::info!("looking for {}", name);
    let mut file = zip_file.by_name(name)?;
    let mut vec = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut vec)?;

    Ok(vec)
}
