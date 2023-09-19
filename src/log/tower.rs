pub(in super::super) fn layer() -> tower_http::trace::TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    trace::Span,
    tower_http::trace::DefaultOnRequest,
    trace::Response,
> {
    tower_http::trace::TraceLayer::new_for_http()
        .on_response(trace::Response)
        .on_failure(tower_http::trace::DefaultOnFailure::new().level(tracing::Level::DEBUG))
        .make_span_with(trace::Span)
}

mod trace {
    #[derive(Copy, Clone)]
    pub struct Response;

    impl<B> tower_http::trace::OnResponse<B> for Response {
        fn on_response(
            self,
            response: &hyper::Response<B>,
            latency: std::time::Duration,
            _: &tracing::Span,
        ) {
            macro_rules! log {
                ($level: expr, $color: literal, $status: ident, $maybe_type: ident, $maybe_length: ident, $latency: ident) => {{
                    let (reason, spacer) = if let Some(reason) = $status.canonical_reason() {
                        (reason, " ")
                    } else {
                        ("", "")
                    };
                    match ($maybe_type, $maybe_length) {
                        (Some(content), Some(length)) => {
                            tracing::event!($level, %content, %length, ?$latency, concat!("{}{}", $color, "{}[m"), reason, spacer, $status.as_u16());
                        }
                        (Some(content), None) => tracing::event!($level, %content, ?$latency, concat!("{}{}", $color, "{}[m"), reason, spacer, $status.as_u16()),
                        (None, Some(length)) => tracing::event!($level, %length, ?$latency, concat!("{}{}", $color, "{}[m"), reason, spacer, $status.as_u16()),
                        (None, None) => tracing::event!($level, ?$latency, concat!("{}{}", $color, "{}[m"), reason, spacer, $status.as_u16()),
                    }
                }}
            }
            let status = response.status();
            let headers = response.headers();
            let maybe_type = headers
                .get(hyper::header::CONTENT_TYPE)
                .and_then(|s| s.to_str().ok());
            let maybe_length = headers
                .get(hyper::header::CONTENT_LENGTH)
                .and_then(|s| s.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|l| *l > 0);
            match status.as_u16() {
                0..=399 => log!(
                    tracing::Level::INFO,
                    "[32m",
                    status,
                    maybe_type,
                    maybe_length,
                    latency
                ),
                400..=499 => log!(
                    tracing::Level::INFO,
                    "[33m",
                    status,
                    maybe_type,
                    maybe_length,
                    latency
                ),
                500..=u16::MAX => log!(
                    tracing::Level::ERROR,
                    "[31m",
                    status,
                    maybe_type,
                    maybe_length,
                    latency
                ),
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct Span;

    impl tower_http::trace::MakeSpan<hyper::Body> for Span {
        #[cfg(not(feature = "log-headers"))]
        fn make_span(&mut self, request: &hyper::Request<hyper::Body>) -> tracing::Span {
            let method = request.method();
            let uri = request.uri();

            #[cfg(feature = "log-headers")]
            macro_rules! log_event {
                ("EXTENSION") => {{
                    let headers = request.headers();
                    tracing::info_span!(target: "", "EXTENSION", message = %uri, %method, ?headers)
                }};
                ($method: literal) => {{
                    let headers = request.headers();
                    tracing::info_span!(target: "", $method, message = %uri, ?headers)
                }};
            }
            #[cfg(not(feature = "log-headers"))]
            macro_rules! log_event {
                ("EXTENSION") => {
                    tracing::info_span!(target: "", "EXTENSION", message = %uri, %method)
                };
                ($method: literal) => {
                    tracing::info_span!(target: "", $method, message = %uri)
                };
            }

            match *method {
                axum::http::Method::OPTIONS => log_event!("OPTIONS"),
                axum::http::Method::GET => log_event!("GET"),
                axum::http::Method::POST => log_event!("POST"),
                axum::http::Method::PUT => log_event!("PUT"),
                axum::http::Method::DELETE => log_event!("DELETE"),
                axum::http::Method::HEAD => log_event!("HEAD"),
                axum::http::Method::TRACE => log_event!("TRACE"),
                axum::http::Method::CONNECT => log_event!("CONNECT"),
                axum::http::Method::PATCH => log_event!("PATCH"),
                _ => log_event!("EXTENSION"),
            }
        }
    }
}
