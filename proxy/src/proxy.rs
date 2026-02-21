use crate::models::{ProxyEvent, ResolvedAction};
use crate::rewrite::apply_transform;
use crate::rules::{normalize_headers, parse_query, resolve_action, select_matching_rule, NormalizedRequest};
use crate::state::AppState;
use anyhow::Context;
use bytes::Bytes;
use chrono::Utc;
use http::header::{HeaderName, HeaderValue};
use http::{Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

type ProxyBody = Full<Bytes>;
type HttpClient = Client<HttpConnector, ProxyBody>;

pub async fn run_proxy_listener(addr: SocketAddr, state: Arc<AppState>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind proxy listener on {}", addr))?;

    info!("proxy listener running on {}", addr);

    let mut connector = HttpConnector::new();
    connector.enforce_http(false);
    let client: HttpClient = Client::builder(TokioExecutor::new()).build(connector);
    let client = Arc::new(client);

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let shared_state = state.clone();
        let shared_client = client.clone();

        tokio::spawn(async move {
            let service = service_fn(move |request| {
                handle_request(request, remote_addr, shared_state.clone(), shared_client.clone())
            });

            if let Err(error) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                warn!("proxy connection error: {error}");
            }
        });
    }
}

async fn handle_request(
    request: Request<Incoming>,
    remote_addr: SocketAddr,
    state: Arc<AppState>,
    client: Arc<HttpClient>,
) -> Result<Response<ProxyBody>, Infallible> {
    let method = request.method().clone();
    let request_uri = request.uri().to_string();

    if method == Method::CONNECT {
        let authority = request
            .uri()
            .authority()
            .map(|value| value.to_string())
            .unwrap_or_default();

        if authority.is_empty() {
            return Ok(simple_response(
                StatusCode::BAD_REQUEST,
                "missing CONNECT authority",
            ));
        }

        let upgrade = upgrade::on(request);
        tokio::spawn(async move {
            match upgrade.await {
                Ok(upgraded) => {
                    if let Err(error) = tunnel(upgraded, authority.clone()).await {
                        warn!("CONNECT tunnel error for {}: {}", authority, error);
                    }
                }
                Err(error) => warn!("upgrade failed: {}", error),
            }
        });

        state
            .record_event(ProxyEvent {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                method: method.to_string(),
                url: request_uri,
                matched_rule_id: None,
                matched_rule_name: None,
                decision: "connect-tunnel".to_string(),
                status: Some(StatusCode::OK.as_u16()),
                notes: vec![format!("remote={}", remote_addr)],
            })
            .await;

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .expect("valid connect response"));
    }

    let result = process_http_request(method.clone(), request, state.clone(), client.clone()).await;

    match result {
        Ok(outcome) => {
            state.record_event(outcome.event).await;
            Ok(outcome.response)
        }
        Err(error) => {
            error!("proxy processing error: {}", error);
            state
                .set_last_error(format!("proxy processing error: {}", error))
                .await;

            state
                .record_event(ProxyEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    method: method.to_string(),
                    url: request_uri,
                    matched_rule_id: None,
                    matched_rule_name: None,
                    decision: "error".to_string(),
                    status: Some(StatusCode::BAD_GATEWAY.as_u16()),
                    notes: vec![error.to_string()],
                })
                .await;

            Ok(simple_response(
                StatusCode::BAD_GATEWAY,
                "interceptkit proxy failed to process request",
            ))
        }
    }
}

struct ProcessedOutcome {
    response: Response<ProxyBody>,
    event: ProxyEvent,
}

async fn process_http_request(
    method: Method,
    request: Request<Incoming>,
    state: Arc<AppState>,
    client: Arc<HttpClient>,
) -> anyhow::Result<ProcessedOutcome> {
    let interception_enabled = state.flags.read().await.interception_enabled;

    let (parts, body) = request.into_parts();
    let target_uri = absolute_uri(&parts.uri, &parts.headers)?;
    let request_body = body.collect().await?.to_bytes();

    let mut outgoing_headers = parts.headers.clone();
    strip_hop_by_hop_headers(&mut outgoing_headers);

    let normalized_request = NormalizedRequest {
        method: method.as_str().to_uppercase(),
        url: target_uri.to_string(),
        headers: normalize_headers(&outgoing_headers),
        query: parse_query(&target_uri.to_string()),
        body_text: String::from_utf8_lossy(&request_body).to_string(),
    };

    let mut notes = vec![];
    let mut matched_rule_id = None;
    let mut matched_rule_name = None;
    let mut decision = "passthrough".to_string();
    let mut outbound_body = Bytes::copy_from_slice(&request_body);

    if interception_enabled {
        let maybe_match = {
            let rules = state.rules.read().await;
            select_matching_rule(&rules, &normalized_request)
        };

        if let Some(candidate) = maybe_match {
            matched_rule_id = Some(candidate.rule.id.clone());
            matched_rule_name = Some(candidate.rule.name.clone());
            notes.extend(candidate.notes);

            let (resolved_action, action_notes) = {
                let mut sequence_counters = state.sequence_counters.write().await;
                resolve_action(&candidate.rule, &mut sequence_counters)
            };
            notes.extend(action_notes);

            match resolved_action {
                ResolvedAction::MockResponse(mock_action) => {
                    if let Some(delay_ms) = mock_action.delay_ms {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        notes.push(format!("delayMs={delay_ms}"));
                    }

                    let body = mock_action.body.unwrap_or_default();
                    let body_bytes = Bytes::from(body.into_bytes());
                    let mut response_builder = Response::builder().status(mock_action.status);

                    if let Some(headers) = response_builder.headers_mut() {
                        for (name, value) in mock_action.headers {
                            if let (Ok(parsed_name), Ok(parsed_value)) = (
                                HeaderName::from_bytes(name.as_bytes()),
                                HeaderValue::from_str(&value),
                            ) {
                                headers.insert(parsed_name, parsed_value);
                            }
                        }

                        if let Some(content_type) = mock_action.content_type {
                            if let Ok(parsed) = HeaderValue::from_str(&content_type) {
                                headers.insert(http::header::CONTENT_TYPE, parsed);
                            }
                        }

                        if let Ok(content_length) =
                            HeaderValue::from_str(&body_bytes.len().to_string())
                        {
                            headers.insert(http::header::CONTENT_LENGTH, content_length);
                        }
                    }

                    decision = "mocked".to_string();
                    let status = StatusCode::from_u16(mock_action.status)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                    let response = response_builder
                        .body(Full::new(body_bytes))
                        .unwrap_or_else(|_| simple_response(status, "failed to build mocked response"));

                    return Ok(ProcessedOutcome {
                        response,
                        event: ProxyEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            timestamp: Utc::now(),
                            method: method.to_string(),
                            url: target_uri.to_string(),
                            matched_rule_id,
                            matched_rule_name,
                            decision,
                            status: Some(status.as_u16()),
                            notes,
                        },
                    });
                }
                ResolvedAction::RewritePassThrough(rewrite_action) => {
                    if let Some(delay_ms) = rewrite_action.delay_ms {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        notes.push(format!("delayMs={delay_ms}"));
                    }

                    if let Some(request_transform) = rewrite_action.request {
                        let request_notes =
                            apply_transform(&mut outgoing_headers, &mut outbound_body, &request_transform)?;
                        notes.extend(request_notes);
                    }

                    let upstream = forward_request(
                        client.clone(),
                        method.clone(),
                        target_uri.clone(),
                        outgoing_headers,
                        outbound_body,
                    )
                    .await?;

                    let mut response_headers = upstream.headers;
                    let mut response_body = upstream.body;

                    if let Some(response_transform) = rewrite_action.response {
                        let response_notes =
                            apply_transform(&mut response_headers, &mut response_body, &response_transform)?;
                        notes.extend(response_notes);
                        decision = "rewritten".to_string();
                    } else {
                        decision = "matched-passthrough".to_string();
                    }

                    let response = build_response(upstream.status, response_headers, response_body);
                    return Ok(ProcessedOutcome {
                        response,
                        event: ProxyEvent {
                            id: uuid::Uuid::new_v4().to_string(),
                            timestamp: Utc::now(),
                            method: method.to_string(),
                            url: target_uri.to_string(),
                            matched_rule_id,
                            matched_rule_name,
                            decision,
                            status: Some(upstream.status.as_u16()),
                            notes,
                        },
                    });
                }
            }
        }
    }

    let upstream = forward_request(
        client,
        method.clone(),
        target_uri.clone(),
        outgoing_headers,
        outbound_body,
    )
    .await?;

    let status = upstream.status;
    let response = build_response(status, upstream.headers, upstream.body);

    Ok(ProcessedOutcome {
        response,
        event: ProxyEvent {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            method: method.to_string(),
            url: target_uri.to_string(),
            matched_rule_id,
            matched_rule_name,
            decision,
            status: Some(status.as_u16()),
            notes,
        },
    })
}

struct UpstreamResponse {
    status: StatusCode,
    headers: http::HeaderMap,
    body: Bytes,
}

async fn forward_request(
    client: Arc<HttpClient>,
    method: Method,
    uri: Uri,
    headers: http::HeaderMap,
    body: Bytes,
) -> anyhow::Result<UpstreamResponse> {
    let mut request_builder = Request::builder().method(method).uri(uri);
    if let Some(request_headers) = request_builder.headers_mut() {
        for (name, value) in &headers {
            request_headers.insert(name, value.clone());
        }
    }

    let request = request_builder
        .body(Full::new(body))
        .context("failed to build upstream request")?;

    let response = client.request(request).await.context("upstream request failed")?;
    let status = response.status();
    let mut response_headers = response.headers().clone();
    strip_hop_by_hop_headers(&mut response_headers);

    let response_body = response.into_body().collect().await?.to_bytes();

    Ok(UpstreamResponse {
        status,
        headers: response_headers,
        body: response_body,
    })
}

fn build_response(status: StatusCode, headers: http::HeaderMap, body: Bytes) -> Response<ProxyBody> {
    let mut response_builder = Response::builder().status(status);
    if let Some(response_headers) = response_builder.headers_mut() {
        for (name, value) in &headers {
            response_headers.insert(name, value.clone());
        }

        if let Ok(content_length) = HeaderValue::from_str(&body.len().to_string()) {
            response_headers.insert(http::header::CONTENT_LENGTH, content_length);
        }
    }

    response_builder
        .body(Full::new(body))
        .unwrap_or_else(|_| simple_response(StatusCode::INTERNAL_SERVER_ERROR, "response build error"))
}

fn simple_response(status: StatusCode, text: &str) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(text.to_string())))
        .expect("simple response is valid")
}

fn absolute_uri(uri: &Uri, headers: &http::HeaderMap) -> anyhow::Result<Uri> {
    if uri.scheme().is_some() && uri.authority().is_some() {
        return Ok(uri.clone());
    }

    let host = headers
        .get(http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .context("missing host header for origin-form request")?;

    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");

    let full = format!("http://{}{}", host, path_and_query);
    full.parse::<Uri>()
        .with_context(|| format!("failed to parse uri {full}"))
}

async fn tunnel(upgraded: hyper::upgrade::Upgraded, authority: String) -> anyhow::Result<()> {
    let mut upgraded = TokioIo::new(upgraded);
    let mut server = TcpStream::connect(&authority)
        .await
        .with_context(|| format!("failed to connect to {authority}"))?;

    tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;
    Ok(())
}

fn strip_hop_by_hop_headers(headers: &mut http::HeaderMap) {
    const HOP_HEADERS: [&str; 8] = [
        "connection",
        "proxy-connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "upgrade",
    ];

    for key in HOP_HEADERS {
        headers.remove(key);
    }
}
