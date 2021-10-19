mod socket_address;

use crate::socket_address::{MultiIncoming, SocketAddress};
use clap::Clap;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Response};
use simple_logger::SimpleLogger;
use std::convert::Infallible;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use atomic_take::AtomicTake;
use futures_channel::oneshot::Sender;
use zip::result::ZipError;
use zip::ZipArchive;

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opts {
    zip_file: PathBuf,
    /// The address list to listen
    #[clap(
        short,
        long,
        multiple_occurrences = true,
        multiple_values = false,
        default_value = ":80"
    )]
    address: Vec<SocketAddress>,
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

    if options.address.is_empty() {
        log::error!("no addresses for listen specified");
        exit(-1);
    }

    let mut incomes = Vec::with_capacity(options.address.len());
    for address in &options.address {
        let incoming = match address.clone().bind() {
            Ok(incoming) => incoming,
            Err(e) => {
                log::error!("can't listen {}: {}", &address, e);
                clean_up(&options.address).await;
                exit(-1);
            }
        };
        incomes.push(incoming);
        log::info!("listening on {}", address);
    }

    let (sender, receiver) = futures_channel::oneshot::channel::<()>();

    handle_signal(sender);

    let server = MultiIncoming::new(incomes)
        .bind_hyper()
        .serve(make_svc)
        .with_graceful_shutdown(async { receiver.await.ok(); });

    log::info!("server started!");
    // Run forever-ish...
    if let Err(err) = server.await {
        log::error!("server error: {}", err);
    }

    clean_up(&options.address).await;
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

async fn clean_up(addresses: &[SocketAddress]) {
    for address in addresses {
        if let Some(err) = address.clean_up().await.err() {
            log::error!("cleanup of {} error: {}", address, err);
        }
    }
}

macro_rules! handle_signal {
    ($sender: ident, $new_handler: expr, $name: expr) => {
        (|| {
            let mut handler = match $new_handler {
                Ok(handler) => handler,
                Err(err) => {
                    log::error!("failed to create signal handler for {}: {}", $name, err);
                    return
                }
            };
            let sender = $sender.clone();
            tokio::spawn(async move {
                while let Some(_) = handler.recv().await {
                    if let Some(sender) = sender.take() {
                        sender.send(()).ok();
                    } else {
                        break
                    };
                }
            });
        })();
    };
}

#[cfg(unix)]
fn handle_signal(sender: Sender<()>) {
    use tokio::signal::unix::{signal, SignalKind};
    let sender = Arc::new(AtomicTake::new(sender));

    handle_signal!(sender, signal(SignalKind::interrupt()), "SIGINT");
    handle_signal!(sender, signal(SignalKind::terminate()), "SIGTERM");
}

#[cfg(windows)]
fn handle_signal(sender: Sender<()>) {
    use tokio::signal::windows::{ctrl_break, ctrl_c};

    let sender = Arc::new(AtomicTake::new(sender));

    handle_signal!(sender, ctrl_c(), "SIGINT");
    handle_signal!(sender, ctrl_break(), "SIGBREAK");
}
