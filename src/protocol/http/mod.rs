mod error;

pub use self::error::Error;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StatusCode {
    /// 400 Bad Request
    BadRequest,

    /// 401 Unauthorized
    Unauthorized,

    /// 402 Payment Required
    PaymentRequired,

    /// 403 Forbidden
    Forbidden,

    /// 404 Not Found
    NotFound,

    /// 501 Not Implemented
    NotImplemented,
}

impl StatusCode {
    pub fn status_line(&self) -> &str {
        match self {
            StatusCode::BadRequest => "HTTP/1.1 400 Bad Request\r\n\r\n",
            StatusCode::Unauthorized => "HTTP/1.1 401 Unauthorized\r\n\r\n",
            StatusCode::PaymentRequired => "HTTP/1.1 402 Payment Required\r\n\r\n",
            StatusCode::Forbidden => "HTTP/1.1 403 Forbidden\r\n\r\n",
            StatusCode::NotFound => "HTTP/1.1 404 Not Found\r\n\r\n",

            StatusCode::NotImplemented => "HTTP/1.1 501 Not Implemented\r\n\r\n",
        }
    }
}
