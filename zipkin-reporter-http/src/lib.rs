#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! A zipkin http reporter allows reporting spans from rust to a zipkin instance via [v2 spans api](https://zipkin.io/zipkin-api/#/default/post_spans). 
//! Spans are buffered in an internal queue and processed in batches. This way reporting is fast and
//! should never block the reporting thread. The actual work can either be done in a background
//! thread or an existing future executor.
//!
//! # Example
//!
//! ```
//! extern crate zipkin;
//! extern crate zipkin_reporter_http;
//! extern crate http;
//! use std::str::FromStr;
//! use zipkin_reporter_http::Builder;
//!
//! // Create a repoter with a dedictaed processing thread.
//! let (_join, reporter) = Builder::new( http::Uri::from_str( "http://zipkin:9411" ).unwrap() )
//!     .start_thread( |e| eprint!["error reporting spans: {}", e] );
//! let tracer = zipkin::Tracer::builder()
//!     .reporter( Box::new( reporter ) )
//!     .build( zipkin::Endpoint::builder().build() );
//! ```

extern crate bytes;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate iovec;
extern crate serde_json;
extern crate tokio;
extern crate zipkin;

use futures::prelude::*;
use futures::sync::mpsc;

use std::thread;
use std::sync::Mutex;
use std::str::FromStr;

mod error;

pub use error::Error;
use error::ErrorInner;

/// A reporter reporting to a zipkin server via http.
///
/// Internally it uses a queue to batch traces and send them in the background.
/// The queue is always bounded to protect against memory shortage.
/// This also means that this reporter may drop spans if it can't report them.
pub struct Reporter {
    sender: Mutex<mpsc::Sender<zipkin::Span>>
}

/// Allows building a Reporter.
#[derive(Debug)]
pub struct Builder<C: hyper::client::connect::Connect> {
    uri: http::Uri,
    client: hyper::client::Client<C,hyper::Body>,
    queue_size: usize,
    chunk_size: usize,
    concurrency: usize,
}

pub(crate) fn resolve_spans_path( uri: http::Uri ) -> http::Uri {
    let mut parts = http::uri::Parts::from( uri );
    parts.path_and_query = match parts.path_and_query {
        Some( ref path_and_query ) => {
            let mut path = path_and_query.path().to_string();
            if !path.ends_with('/') {
                path.push_str("/")
            }
            path.push_str("api/v2/spans");
            if let Some(query) = path_and_query.query().as_ref() {
                path.push_str("?");
                path.push_str(query);
            };
            Some( http::uri::PathAndQuery::from_str(&path).unwrap() )
        },
        None => {
            Some( http::uri::PathAndQuery::from_static("/api/v2/spans") )
        }
    };
    http::Uri::from_parts( parts ).expect("Invalid Uri supplied to zipkin_reporter_http::Builder::new")
}

impl Builder<hyper::client::HttpConnector> {

    /// Starts building a new client using the supplied Uri.
    pub fn new( uri : http::Uri ) -> Self {
        Builder{
            uri: resolve_spans_path( uri ),
            queue_size: 100,
            chunk_size:  20,
            concurrency: 5,
            client: hyper::Client::builder().build_http()
        }
    }
}

impl<C: hyper::client::connect::Connect> Builder<C> {

    /// Sets the chunk size of this reporter.
    /// 
    /// The reporter delays reporting until this number of spans are collected.
    ///
    /// # Panics
    ///
    /// You cannot set the chunk size to 0. This method panics if you try to.
    pub fn chunk_size( mut self, chunk_size: usize ) -> Self {
        if chunk_size == 0 {
            panic!["chunk_size must be at least 1"];
        }
        self.chunk_size = chunk_size;
        self
    }

    /// Sets the queue size of this reporter.
    /// 
    /// This queue buffers spans until the background reporter has picked them up.
    ///
    /// # Warning
    /// 
    /// Setting this to 0 is possible but will make the reporter lossy.
    pub fn queue_size( mut self, queue_size: usize ) -> Self {
        self.queue_size = queue_size;
        self
    }

    /// Sets the concurrency of this reporter.
    ///
    /// The concurrency is the number of parallel requests that are issued.
    ///
    /// # Panics
    ///
    /// You cannot set the concurrency to 0. This method panics if you try to.
    pub fn concurrency( mut self, concurrency: usize ) -> Self {
        if concurrency == 0 {
            panic!["concurrency must at least be 1"];
        }
        self.concurrency = concurrency;
        self
    }

    /// Changes the http client used to send the spans.
    ///
    /// This mainly allows changing the connector.
    pub fn client<D: hyper::client::connect::Connect> ( self, client: hyper::Client<D, hyper::Body> ) -> Builder<D> {
        Builder{ client, uri: self.uri, concurrency: self.concurrency, chunk_size: self.chunk_size, queue_size: self.queue_size }
    }

}

/// Worker implements the logic for sending spans in the background.
///
/// A worker is always created together with a reporter and dispatches 
/// spans from the internal queue to the actual zipkin instance. In order 
/// to actually do something it has to be spawned on a future executor.
#[must_use = "Worker must be polled in order to actually send spans."]
pub struct Worker {
    inner: Box<Stream<Item=(),Error=Error> + Send>
}

impl std::fmt::Debug for Worker {
    
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Worker").finish()
    }
}

impl Stream for Worker {
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>,Self::Error> {
        self.inner.poll()
    }
}

impl<C> Builder<C>
    where
        C: hyper::client::connect::Connect + 'static,
        C::Future: 'static
    {

    /// Creates a new reporter and a stream containing the 
    /// background reporter.
    ///
    /// This method can be used to control error handling and 
    /// scheduling directly.
    ///
    /// # Example
    /// 
    /// ```
    /// extern crate futures;
    /// extern crate http;
    /// extern crate tokio;
    /// extern crate zipkin;
    /// extern crate zipkin_reporter_http;
    /// use std::str::FromStr;
    /// use futures::prelude::*;
    /// use zipkin_reporter_http::Builder;
    ///
    /// // Create a reporter and a stream of errors.
    /// let (stream, reporter) = Builder::new( http::Uri::from_str("http://zipkin:9411").unwrap() ).build();
    /// 
    /// // Run the background processor and the reporter on the same tokio executor.
    /// tokio::run(futures::lazy(move ||{
    ///     // Spawn the background processor.
    ///     tokio::spawn( stream
    ///         .map_err(|e| eprint!["error reporting spans {}", e] )
    ///         .for_each(|_| Ok(()) ) );
    ///     
    ///     // Create a tracer.
    ///     let _tracer = zipkin::Tracer::builder()
    ///         .reporter( Box::new( reporter ) )
    ///         .build( zipkin::Endpoint::builder()
    ///             .service_name("zipkin_reporter_http test")
    ///             .build() );
    ///     Ok(())
    /// }))
    /// ```
    pub fn build(self) -> ( Worker, Reporter ) {
        let Builder{ uri, client, queue_size, chunk_size, concurrency } = self;
        let (sender, receiver) = mpsc::channel( queue_size );
        let worker_inner = receiver.chunks( chunk_size )
            .map_err(|_| unreachable!() )
            .filter_map(|spans|{
                match serde_json::to_string( &spans ) {
                    Ok(body) => Some(body),
                    Err(err) => {
                        eprint!["zipkin-reporter-http: failed to serialize span ( {} ).\n\tThis is probably a bug. Please file a bug report against https://github.com/palantir/rust-zipkin\n", err ];
                        None
                    }
                }
            })
            .map(move |body|{
                let request = hyper::Request::builder()
                    .method( http::method::Method::POST )
                    .header( http::header::CONTENT_TYPE, http::header::HeaderValue::from_static( "application/json" ) )
                    .uri( uri.clone() )
                    .body( hyper::Body::from( body ) ).expect( "http request" );
                client.request( request ).then( |response|{
                    match response {
                        Ok( r ) => {
                            if r.status().is_success() {
                                Ok( () )
                            } else {
                                Err( Error{ inner: ErrorInner::Http( r.status() ) } )
                            }
                        },
                        Err( e ) => {
                            Err( Error{ inner: ErrorInner::Hyper(e) } )
                        }
                    }
                } )
            } ).buffer_unordered( concurrency );
        ( Worker{ inner: Box::new(worker_inner) }, Reporter{ sender: Mutex::new( sender ) } )
    }

    /// Builds the reporter and creates a background thread.
    ///
    /// # Panics
    /// When the OS fails to create the backing thread this method panics.
    pub fn start_thread<F>( self, error_handler: F ) -> (thread::JoinHandle<()>, Reporter)
        where F: Send + Fn(Error) + 'static {
        let (worker, reporter) = self.build();
        let handle = thread::Builder::new()
            .name("zipkin-reporter-http".to_string())
            .spawn(move ||{
                hyper::rt::run(
                    worker
                        .map_err(error_handler)
                        .for_each(|_|{ Ok(()) })
                );
            }).unwrap();
        (handle, reporter)
   }

}

impl zipkin::Report for Reporter {

    fn report2(&self, span: zipkin::Span) {
        if self.sender.lock().unwrap().try_send( span ).is_err() {
            eprint!["zipkin-reporter-http: failed to queue span\n"]
        }
    }

}

#[cfg(test)]
mod test {

    use zipkin;
    use zipkin::Report;
    use super::*;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;
    use std::sync::mpsc;

    fn test_server<F> (port: u16, responder: F ) -> mpsc::Receiver<hyper::Request<Vec<u8>>> where
        F: 'static + Send + Clone + Fn( &hyper::Request<Vec<u8>>) -> hyper::Response<hyper::Body>
        {
        let (tx, rx) = mpsc::sync_channel(10);
        let server = hyper::Server::bind( &([127u8,0,0,1],port).into() )
            .serve(move ||{
                let tx = tx.clone();
                let responder = responder.clone();
                hyper::service::service_fn(move |req : hyper::Request<hyper::Body>|{
                    let (head, body) = req.into_parts();
                    let tx = tx.clone();
                    let responder = responder.clone();
                    body.concat2().and_then(move |content|{
                        let req = http::Request::from_parts(head, content.to_vec());
                        let response = responder( &req );
                        tx.send( req ).unwrap();
                        Ok(response)
                    })
                })
            });
        thread::spawn(move ||{
            hyper::rt::run(server.map_err(|e| eprint!["{:?}", e]))
        });
        return rx;
    }

    fn test_error_handler() -> ( mpsc::Receiver<Error>, Box<Fn(Error) + Send + 'static> ) {
        let (tx, rx) = mpsc::sync_channel(10);
        return (rx, Box::new( move |err: Error|{ tx.send(err).unwrap() } ) )
    }

    #[test]
    fn it_should_report() {
        let rx = test_server( 19411, |_| hyper::Response::builder()
                           .status(http::StatusCode::ACCEPTED)
                           .body( hyper::Body::from("Ok") ).unwrap() );
        let (erx, eh) = test_error_handler();
        let (_, reporter) = Builder::new( http::Uri::from_str( "http://localhost:19411" ).unwrap() )
            .chunk_size( 1 )
            .start_thread( move |e| (*eh)(e) );

        // WHEN
        let span = zipkin::Span::builder()
            .id( zipkin::SpanId::from( [0 as u8,0,0,0,0,0,0,1] ) )
            .trace_id( zipkin::TraceId::from([0 as u8,0,0,0,0,0,0,0]) )
            .name( "foo" )
            .kind( zipkin::Kind::Client )
            .duration( Duration::from_secs( 1 ) )
            .build();
        reporter.report2( span.clone() );
        // THEN
        let req : hyper::Request<Vec<u8>> = rx.recv().unwrap();
        assert_eq![ req.uri().path() , "/api/v2/spans" ];
        assert_eq![ req.method(), &http::Method::POST ];
        let mut body = Vec::with_capacity(128);
        body.push( b'[' );
        serde_json::to_writer(&mut body, &span).unwrap();
        body.push( b']' );
        assert_eq![ req.body(), &body ];
        assert_eq![ req.headers().get("Content-Length"), Some(&hyper::header::HeaderValue::from(body.len())) ];
        assert_eq![ erx.try_recv().unwrap_err(), mpsc::TryRecvError::Empty ];

        // CLEANUP
        drop( reporter );
    }


    #[test]
    fn it_should_call_the_error_handler() {
        let _rx = test_server( 19412, |_| hyper::Response::builder()
                           .status(http::StatusCode::FORBIDDEN)
                           .body( hyper::Body::from("Forbidden") ).unwrap() );
        let (erx, eh) = test_error_handler();
        let (_, reporter) = Builder::new( http::Uri::from_str( "http://localhost:19412/" ).unwrap() )
            .chunk_size( 1 )
            .start_thread( move |e| (*eh)(e) );

        // WHEN
        let span = zipkin::Span::builder()
            .id( zipkin::SpanId::from( [0 as u8,0,0,0,0,0,0,1] ) )
            .trace_id( zipkin::TraceId::from([0 as u8,0,0,0,0,0,0,0]) )
            .name( "foo" )
            .kind( zipkin::Kind::Client )
            .duration( Duration::from_secs( 1 ) )
            .build();
        reporter.report2( span.clone() );
        // THEN
        let err = erx.recv().unwrap();
        assert_eq![ err.status_code(), Some(http::StatusCode::FORBIDDEN) ];

        // CLEANUP
        drop( reporter );
    }

    #[test]
    fn it_resolves_the_spans_path() {
        assert_eq![
            resolve_spans_path( http::Uri::from_str("http://localhost").unwrap() ),
            http::Uri::from_str("http://localhost/api/v2/spans").unwrap()
        ];
        assert_eq![
            resolve_spans_path( http::Uri::from_str("http://localhost/").unwrap() ),
            http::Uri::from_str("http://localhost/api/v2/spans").unwrap()
        ];
        assert_eq![
            resolve_spans_path( http::Uri::from_str("http://localhost/sub").unwrap() ),
            http::Uri::from_str("http://localhost/sub/api/v2/spans").unwrap()
        ];
        assert_eq![
            resolve_spans_path( http::Uri::from_str("http://localhost/sub/").unwrap() ),
            http::Uri::from_str("http://localhost/sub/api/v2/spans").unwrap()
        ];
        assert_eq![
            resolve_spans_path( http::Uri::from_str("http://localhost/?query=param").unwrap() ),
            http::Uri::from_str("http://localhost/api/v2/spans?query=param").unwrap()
        ];
    }

}
