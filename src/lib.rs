//! Lined-oriented Rustlang
//! [Streams](http://alexcrichton.com/futures-rs/futures/stream/index.html)
//!
//! Streams represent spools of results computed asyncronously. They are the async
//! analog to Rustlang's Iterator type.
//!
//! This crate represents a Stream transformer over Stream's of `AsRef<[u8]>`
//! that result in Stream's of line-oriented values. Chunks of bytes are delimted
//! by LF (\n) and optionally CRLF (\r\n) patterns.
//!
//! You may find this crate useful if you are interfacing with a line-oriented
//! protocol transmited over a network connection. Networks may split your protocol
//! into uneven chunks of bytes. This crate buffers those chunks and yields
//! a Stream that reconnects those lines.
//!
//! # Examples
//!
//!  The underlying abstraction is the [Lines](struct.Lines.html) type which
//!   return's a stream over an arbitrarily converted type, but most
//!  applications will typically want to use the `strings` method which return's
//!  a Stream of Strings
//!
//! ```no_run
//! extern crate futures;
//! extern crate stream_lines;
//! extern crate tokio;
//!
//! use std::string::FromUtf8Error;
//!
//! use futures::stream::iter_ok;
//! use futures::Stream;
//! use tokio::runtime::Runtime;
//!
//! fn main() {
//!     let chunks = vec![
//!        "\nhello ",
//!         "world\n",
//!         "\n",
//!         "what a\nlovely",
//!         "\nday\n"
//!      ];
//!     let stream = iter_ok::<_, FromUtf8Error>(chunks);
//!     let print =
//!         stream_lines::strings(stream)
//!            .for_each(|line| Ok(println!("{}", line)));
//!     Runtime::new().expect("failed to initialize runtime").block_on(print).expect("failed to complete stream");
//! }
//! ```
#![deny(missing_docs)]

extern crate futures;

use std::mem::replace;
use std::string::FromUtf8Error;

use futures::stream::Fuse;
use futures::{Async, Poll, Stream};

const LF: u8 = b'\n';
const CR: u8 = b'\r';

/// Converts a fused `Stream` of bytes into a line-oriented stream
/// of a target type
pub struct Lines<S: Stream, O, E> {
    buffered: Option<Vec<u8>>,
    stream: Fuse<S>,
    into: fn(Vec<u8>) -> Result<O, E>,
}

/// A lined oriented stream of `Strings`
pub fn strings<S>(s: S) -> Lines<S, String, FromUtf8Error>
where
    S: Stream,
{
    Lines::new(s, String::from_utf8)
}

impl<S: Stream, O, E> Lines<S, O, E> {
    /// Creates a new `Lines` instance that wraps another stream
    pub fn new(
        stream: S,
        into: fn(Vec<u8>) -> Result<O, E>,
    ) -> Self {
        Lines {
            buffered: None,
            stream: stream.fuse(),
            into: into,
        }
    }

    fn next(
        &mut self,
        flush: bool,
    ) -> Option<Result<O, E>> {
        let buffered = replace(&mut self.buffered, None);
        if let Some(ref buffer) = buffered {
            let mut split = buffer.splitn(2, |c| *c == LF);
            if let Some(first) = split.next() {
                let mut line = first.to_vec();
                if let Some(&CR) = line.last() {
                    line.pop();
                }
                if let Some(second) = split.next() {
                    replace(&mut self.buffered, Some(second.to_vec()));
                    return Some((self.into)(line));
                } else if flush {
                    return Some((self.into)(line));
                }
            }
        }
        replace(&mut self.buffered, buffered);
        None
    }
}

/// This implementation should be flexible enough to work with plan strings as well as `hyper::Chunks`
/// Errors from the underlying stream should be able to convert parse errors into the streams native
/// error type using a `From` impl. this changed between hyper 0.11 and 0.12.
impl<S, O, E> Stream for Lines<S, O, E>
where
    S: Stream,
    S::Item: AsRef<[u8]>,
    S::Error: From<E>,
{
    type Item = O;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<O>, S::Error> {
        match self.stream.poll()? {
            Async::NotReady => Ok(Async::NotReady),
            Async::Ready(None) => match self.next(true) {
                Some(Ok(line)) => Ok(Async::Ready(Some(line))),
                Some(Err(err)) => Err(err.into()),
                None => Ok(Async::Ready(None)),
            },
            Async::Ready(Some(chunk)) => {
                if let Some(ref mut buffer) = self.buffered {
                    buffer.extend(chunk.as_ref());
                } else {
                    self.buffered = Some(chunk.as_ref().to_vec());
                }
                match self.next(false) {
                    Some(Ok(line)) => Ok(Async::Ready(Some(line))),
                    Some(Err(err)) => Err(err.into()),
                    None => Ok(Async::NotReady),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::iter_ok;
    #[test]
    fn it_delimits_by_lf() {
        let chunks = vec!["hello ", "world\n", "\n", "what a\nlovely", "\nday\n"];
        let stream = iter_ok::<_, FromUtf8Error>(chunks);
        let mut lines = strings(stream);
        assert_eq!(lines.poll().unwrap(), Async::NotReady);
        assert_eq!(
            lines.poll().unwrap(),
            Async::Ready(Some("hello world".into()))
        );
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("what a".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("lovely".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("day".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(None));
    }

    #[test]
    fn it_delimits_by_crlf() {
        let chunks = vec![
            "hello ",
            "world\r\n",
            "\r\n",
            "what a\r\nlovely",
            "\r\nday\r\n",
        ];
        let stream = iter_ok::<_, FromUtf8Error>(chunks);
        let mut lines = strings(stream);
        assert_eq!(lines.poll().unwrap(), Async::NotReady);
        assert_eq!(
            lines.poll().unwrap(),
            Async::Ready(Some("hello world".into()))
        );
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("what a".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("lovely".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("day".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(Some("".into())));
        assert_eq!(lines.poll().unwrap(), Async::Ready(None));
    }
}
