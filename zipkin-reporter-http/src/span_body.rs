use bytes;
use hyper;
use futures::{Async, Poll};
use iovec::IoVec;

/// A SpanBody contains a chunk of spans.
///
/// This type allows for zero-copy concatenation of spans.
#[derive(Debug)]
pub struct SpanBody {
    len: u64,
    spans: Option<SpanBuf>
}

#[derive(Clone,Copy,Debug)]
enum SpanBufState {
    Empty,
    Before{ span: usize },
    Inside{ span: usize, offset: usize },
    Closing,
    Terminal
}

#[derive(Debug)]
pub struct SpanBuf {
    state: SpanBufState,
    spans: Vec<Vec<u8>>
}

static OPEN : [u8; 1] = [b'['];
static COMMA : [u8; 1] = [b','];
static CLOSE : [u8; 1] = [b']'];
static TERMINAL : [u8; 0] = [];

impl bytes::Buf for SpanBuf {

    fn remaining(&self) -> usize {
        match self.state {
            SpanBufState::Empty => {
                2
            },
            SpanBufState::Before{ span } => {
                let sum : usize = self.spans[ span.. ].iter().map(|s| s.len() ).sum();
                sum + self.spans.len() - span + 1
            },
            SpanBufState::Inside{ span, offset } => {
                let sum : usize = self.spans[ span.. ].iter().map(|s| s.len() ).sum();
                sum - offset + self.spans.len() - span
            },
            SpanBufState::Closing => 1,
            SpanBufState::Terminal => 0
        }
    }

    fn bytes(&self) -> &[u8] {
        match self.state {
            SpanBufState::Empty => {
                &OPEN
            },
            SpanBufState::Before{ span } => {
                if span == 0 {
                    &OPEN
                } else {
                    &COMMA
                }
            },
            SpanBufState::Inside{ span, offset } => {
                &self.spans[ span ][ offset.. ]
            },
            SpanBufState::Closing => {
                &CLOSE
            },
            SpanBufState::Terminal => {
                &TERMINAL
            }
        }
    }

    fn advance(&mut self, cnt: usize) {
        let mut remaining = cnt;
        while remaining > 0 {
            let (consumed, next) = match self.state {
                SpanBufState::Empty => {
                    ( 1, SpanBufState::Closing )
                },
                SpanBufState::Before{ span } => {
                    ( 1, SpanBufState::Inside{ span, offset: 0 } )
                },
                SpanBufState::Inside{ span, offset } => {
                    let vec = &self.spans[ span ];
                    if vec.len() - offset > remaining {
                        ( remaining, SpanBufState::Inside{ span, offset: offset + remaining } )
                    } else if span + 1 == self.spans.len() {
                        ( vec.len() - offset, SpanBufState::Closing )
                    } else {
                        ( vec.len() - offset, SpanBufState::Before{ span: span + 1 } )
                    }
                },
                SpanBufState::Closing => {
                    ( 1, SpanBufState::Terminal )
                },
                SpanBufState::Terminal => {
                    panic!["advance( {} ) is {} past the end", cnt, remaining ]
                }
            };
            remaining -= consumed;
            self.state = next;
        }
    }

    fn bytes_vec<'a>(&'a self, dst: &mut [&'a IoVec]) -> usize {
        let mut i = 0;
        let mut state = self.state;
        for iovec in dst.iter_mut() {
            let next = match state {
                SpanBufState::Empty => {
                    *iovec = (&OPEN[..]).into();
                    SpanBufState::Closing
                },
                SpanBufState::Before{ span } => {
                    if span == 0 {
                        *iovec = (&OPEN[..]).into();
                    } else {
                        *iovec = (&COMMA[..]).into();
                    }
                    SpanBufState::Inside{ span, offset: 0 }
                },
                SpanBufState::Inside{ span, offset } => {
                    *iovec = self.spans[ span ][ offset.. ].into();
                    let next_span = span + 1;
                    if next_span == self.spans.len() {
                        SpanBufState::Closing
                    } else {
                        SpanBufState::Before{ span: next_span }
                    }
                },
                SpanBufState::Closing => {
                    *iovec = (&CLOSE[..]).into();
                    SpanBufState::Terminal
                },
                SpanBufState::Terminal => {
                    break;
                }
            };
            state = next;
            i+=1;
        }
        i
    }

}

impl SpanBody {
    pub(crate) fn new(vec: Vec<Vec<u8>>) -> Self {
        if vec.is_empty() {
            SpanBody{
                len: 2,
                spans: Some(SpanBuf{ spans: vec, state: SpanBufState::Empty })
            }
        } else {
            let vec_len : usize = vec.iter().map(|s| s.len() ).sum();
            SpanBody{
                len: (vec_len + vec.len() + 1) as u64,
                spans: Some(SpanBuf{ spans: vec, state: SpanBufState::Before{ span: 0 } })
            }
        }
    }
}

impl hyper::body::Payload for SpanBody {
    type Data = SpanBuf;
    type Error = hyper::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>,Self::Error> {
        Ok( Async::Ready( self.spans.take() ) )
    }

    fn is_end_stream(&self) -> bool {
        self.spans.is_none()
    }

    fn content_length(&self) -> Option<u64> {
        Some( self.len )
    }

}

#[cfg(test)]
mod test {

    use std::mem;
    use super::*;
    use bytes::Buf;
    use hyper::body::Payload;


    fn collect_iovec<B: bytes::Buf>(b: &B) -> Vec<u8> {
        let mut vecs : [&IoVec; 16] = unsafe{ mem::uninitialized() };
        let n = b.bytes_vec(&mut vecs);
        return vecs[0..n].iter().flat_map(|r| r[..].iter() ).cloned().collect::<Vec<u8>>();
    }

    #[test]
    fn it_works_correctly_with_empty_bodies() {
        let mut body = SpanBody::new( vec![] );
        assert_eq![ body.content_length(), Some(2) ];
        let data = body.poll_data();
        if let Ok(Async::Ready(Some(buf))) = data {
            assert_eq![ buf.remaining(), 2 ];
            assert_eq![ collect_iovec( &buf ), vec![b'[',b']'] ];
            assert_eq![ buf.iter().collect::<Vec<_>>(), vec![b'[',b']'] ];
        } else {
            panic!["Unexpected data: {:?}", data];
        }
        assert_eq![ body.is_end_stream(), true ];
    }

    #[test]
    fn it_works_correctly_with_one_span() {
        let mut body = SpanBody::new( vec![vec![b'{',b'}']] );
        assert_eq![ body.content_length(), Some(4) ];
        let data = body.poll_data();
        if let Ok(Async::Ready(Some(buf))) = data {
            assert_eq![ buf.remaining(), 4 ];
            assert_eq![ collect_iovec( &buf ), vec![b'[',b'{',b'}',b']'] ];
            assert_eq![ buf.iter().collect::<Vec<_>>(), vec![b'[',b'{',b'}',b']'] ];
        } else {
            panic!["Unexpected data: {:?}", data];
        }
    }

    #[test]
    fn it_works_correctly_with_multiple_spans() {
        let mut body = SpanBody::new( vec![vec![b'{',b'}'],vec![b'{',b'}'],vec![b'{',b'}']] );
        assert_eq![ body.content_length(), Some(10) ];
        let data = body.poll_data();
        if let Ok(Async::Ready(Some(buf))) = data {
            assert_eq![ buf.remaining(), 10 ];
            assert_eq![ collect_iovec( &buf ), vec![
                b'[',b'{',b'}',
                b',',b'{',b'}',
                b',',b'{',b'}',
                b']'
            ] ];
            assert_eq![ buf.iter().collect::<Vec<_>>(), vec![
                b'[',b'{',b'}',
                b',',b'{',b'}',
                b',',b'{',b'}',
                b']'
            ] ];
        } else {
            panic!["Unexpected data: {:?}", data];
        }
    }
}
