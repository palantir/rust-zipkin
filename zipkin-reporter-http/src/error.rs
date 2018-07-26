use hyper;
use http;
use std::fmt;
use std::error;

/// Error type for the zipkin http reporter.
#[derive(Debug)]
pub struct Error {
    pub(crate) inner: ErrorInner
}

#[derive(Debug)]
pub(crate) enum ErrorInner {
    Hyper( hyper::Error ),
    Http( http::StatusCode )
}

impl Error {

    /// True if the error was http related. Usually this means an unexpected response from the
    /// zipkin server.
    pub fn is_http_error(&self) -> bool {
        match self.inner {
            ErrorInner::Http( _ ) => true,
            _ => false
        }
    }

    /// True if the error was hyper related.
    pub fn is_hyper_error(&self) -> bool {
        match self.inner {
            ErrorInner::Hyper( _ ) => true,
            _ => false
        }
    }

    /// HTTP status code from the zipkin server if there was any.
    pub fn status_code(&self) -> Option<http::StatusCode> {
        match self.inner {
            ErrorInner::Http( status ) => Some( status ),
            _ => None
        }
    }

}

impl fmt::Display for Error {
    
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.inner {
            ErrorInner::Hyper( ref e ) => {
                fmt::Display::fmt( e, f )
            },
            ErrorInner::Http( c ) => {
                write![f, "zipkin server replied with status code {}", c]
            }
        }
    }
}

impl error::Error for Error {

    fn cause(&self) -> Option<&error::Error> {
        match self.inner {
            ErrorInner::Hyper( ref e ) => {
                Some( e )
            },
            _ => None
        }
    }

}

#[cfg(test)]
mod test {

    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn it_works_for_hyper_errors() {
        // This seems to be the easiest way to get a hyper error.
        let (mut sender, body) = hyper::body::Body::channel();
        drop( body );
        let err = Error{ inner: ErrorInner::Hyper( sender.poll_ready().unwrap_err() ) };
        assert![ err.is_hyper_error() ];
        assert_eq![ err.to_string(), "connection closed" ];
        let cause = err.cause();
        assert![ cause.is_some() ];
        assert_eq![ cause.unwrap().description(), "connection closed" ];
    }

    #[test]
    fn it_works_for_status_code() {
        let err = Error{ inner: ErrorInner::Http( http::StatusCode::INTERNAL_SERVER_ERROR ) };
        assert![ err.is_http_error() ];
        assert_eq![ err.to_string(), "zipkin server replied with status code 500 Internal Server Error" ];
        assert![ err.cause().is_none() ];
        assert_eq![ err.status_code(), Some( http::StatusCode::INTERNAL_SERVER_ERROR ) ];
    }

}
