extern crate futures;
extern crate stream_lines;
extern crate tokio;

use std::string::FromUtf8Error;

use futures::stream::iter_ok;
use futures::Stream;
use tokio::runtime::Runtime;

fn main() {
    let chunks = vec!["\nhello ", "world\n", "\n", "what a\nlovely", "\nday\n"];
    let stream = iter_ok::<_, FromUtf8Error>(chunks);
    let print = stream_lines::strings(stream).for_each(|line| Ok(println!("{}", line)));
    Runtime::new()
        .expect("failed to initialize runtime")
        .block_on(print)
        .expect("failed to execute stream");
}
