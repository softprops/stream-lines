extern crate futures;
extern crate stream_lines;
extern crate tokio_core;

use std::string::FromUtf8Error;

use futures::Stream;
use futures::stream::iter_ok;
use tokio_core::reactor::Core;

fn main() {
    let mut core = Core::new().unwrap();
    let chunks = vec!["\nhello ", "world\n", "\n", "what a\nlovely", "\nday\n"];
    let stream = iter_ok::<_, FromUtf8Error>(chunks);
    let print =
        stream_lines::strings(stream).for_each(|line| Ok(println!("{}", line)));
    core.run(print).unwrap();
}