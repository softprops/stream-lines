extern crate hyper;
extern crate stream_lines;
extern crate futures;
extern crate tokio_core;
extern crate hyper_tls;

use futures::{Future, Stream};
use hyper::{Client, Method};
use hyper::client::Request;
use hyper::header::{Accept, qitem};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

fn run() -> Result<(), hyper::Error> {
    let mut core = Core::new()?;
    let connector = HttpsConnector::new(4, &core.handle()).unwrap();
    let http = Client::configure()
        .connector(connector)
        .keep_alive(true)
        .build(&core.handle());

    let mut req = Request::new(
        Method::Get,
        "https://stream.wikimedia.org/v2/stream/recentchange"
            .parse()?,
    );
    req.headers_mut().set(Accept(vec![
        qitem(
            "text/event-stream".parse().unwrap()
        ),
    ]));
    let work = http.request(req).and_then(|resp| {
        stream_lines::strings(resp.body()).for_each(|line| {
            Ok(println!("-> {}", line))
        })
    });
    core.run(work).map_err(From::from)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e)
    }
}
