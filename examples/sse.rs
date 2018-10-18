#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate stream_lines;
extern crate tokio;

use std::error::Error;
use std::string::FromUtf8Error;

use failure::Fail;
use futures::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use tokio::runtime::Runtime;

#[derive(Debug, Fail)]
enum AppErr {
    #[fail(display = "encoding error: {}", _0)]
    Utf8(#[cause] FromUtf8Error),
    #[fail(display = "http error: {}", _0)]
    Http(#[cause] hyper::Error),
}

impl From<FromUtf8Error> for AppErr {
    fn from(e: FromUtf8Error) -> Self {
        AppErr::Utf8(e)
    }
}

impl From<hyper::Error> for AppErr {
    fn from(e: hyper::Error) -> Self {
        AppErr::Http(e)
    }
}

fn run() -> Result<(), Box<Error>> {
    let connector = HttpsConnector::new(4).expect("unable to initialize https connector");
    let http = Client::builder()
        .keep_alive(true)
        .build::<_, Body>(connector);

    let req = Request::get("https://stream.wikimedia.org/v2/stream/recentchange")
        .header("Accept", "text/event-stream")
        .body(Default::default())?;
    let work = http.request(req).map_err(AppErr::from).and_then(|resp| {
        stream_lines::strings(resp.into_body().map_err(AppErr::from))
            .for_each(|line| Ok(println!("-> {}", line)))
    });
    Runtime::new()
        .expect("failed to initialize runtime")
        .block_on(work)
        .map_err(AppErr::compat)?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e)
    }
}
