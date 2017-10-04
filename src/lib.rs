//! Lined-oriented Rustlang [Streams](http://alexcrichton.com/futures-rs/futures/stream/index.html)
//!
//! Streams represent spools of results computed asyncronously. They are the async
//! analog to Rustlang's Iterator type
//!
//! This crate represents a Stream transformer over Stream's of `AsRef<[u8]>`
//! that result in Stream's of line-oriented results.
//!

#![warn(missing_docs)]

extern crate futures;

use std::mem::replace;
use std::string::FromUtf8Error;

use futures::{Async, Poll, Stream};
use futures::stream::Fuse;

const NEWLINE: u8 = b'\n';

/// Converts to a fused stream into a line oriented stream
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
    pub fn new(stream: S, into: fn(Vec<u8>) -> Result<O, E>) -> Self {
        Lines {
            buffered: None,
            stream: stream.fuse(),
            into: into,
        }
    }

    fn next(&mut self, flush: bool) -> Option<Result<O, E>> {
        let buffered = replace(&mut self.buffered, None);
        if let Some(ref buffer) = buffered {
            let mut split = buffer.splitn(2, |c| *c == NEWLINE);
            if let Some(first) = split.next() {
                if let Some(second) = split.next() {
                    replace(&mut self.buffered, Some(second.to_vec()));
                    return Some((self.into)(first.to_vec()));
                } else if flush {
                    return Some((self.into)(first.to_vec()));
                }
            }
        }
        replace(&mut self.buffered, buffered);
        None
    }
}

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
            Async::Ready(None) => {
                match self.next(true) {
                    Some(Ok(line)) => Ok(Async::Ready(Some(line))),
                    Some(Err(err)) => Err(err.into()),
                    None => Ok(Async::Ready(None)),
                }
            }
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
    fn it_works() {
        let chunks =
            vec!["hello ", "world\n", "\n", "what a\nlovely", "\nday\n"];
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
