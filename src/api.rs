use flate2::{write::GzEncoder, Compression};
use futures::join;
use reqwest::{
    header::{CONTENT_ENCODING, CONTENT_TYPE},
    Client, RequestBuilder,
};
use serde::Serialize;
use std::cmp::max;
use std::time::Duration;
use tokio::time::sleep;

use super::types::{NrLog, NrSpan};

pub enum ApiEndpoint {
    US,
    EU,
    Custom(String),
}

impl Default for ApiEndpoint {
    fn default() -> Self {
        ApiEndpoint::US
    }
}

#[derive(Default)]
pub struct Api {
    pub log_endpoint: ApiEndpoint,
    pub trace_endpoint: ApiEndpoint,
    pub key: String,
    pub client: Client,
}

impl Api {
    pub(crate) async fn send(&self, logs: Vec<Logs>, traces: Vec<Spans>) {
        let mut logs_service = TracesService::new(logs);
        let mut trace_service = TracesService::new(traces);

        loop {
            use ServiceStatus::*;

            match join!(logs_service.send(self), trace_service.send(self)) {
                (Timeount(d1), Timeount(d2)) => sleep(max(d1, d2)).await,

                (Timeount(d), _) | (_, Timeount(d)) => sleep(d).await,

                (Finished, Finished) => return,

                _ => {}
            }
        }
    }
}

enum ServiceStatus {
    // Need to wait before next sending
    Timeount(Duration),

    // Have remaining data to be sent
    Remaining,

    // Finished, either success or failed
    Finished,
}

struct TracesService<T: Sendable> {
    data: Vec<T>,
    // number of items to send each request,
    batch_len: usize,
    retry_count: u32,
}

impl<T: Sendable> TracesService<T> {
    fn new(data: Vec<T>) -> Self {
        TracesService {
            batch_len: data.len(),
            data,
            retry_count: 0,
        }
    }

    async fn send(&mut self, api: &Api) -> ServiceStatus {
        // nothing to send
        if self.data.is_empty() {
            return ServiceStatus::Finished;
        }

        let res = T::build_request(&self.data[0..self.batch_len], api)
            .send()
            .await
            .unwrap();

        // https://docs.newrelic.com/docs/distributed-tracing/trace-api/trace-api-general-requirements-limits#status-codes
        match res.status().as_u16() {
            // success
            200..=299 => {
                // reset retry_count
                self.retry_count = 0;

                self.data.drain(0..self.batch_len);

                if self.data.is_empty() {
                    ServiceStatus::Finished
                } else {
                    ServiceStatus::Remaining
                }
            }

            400 | 401 | 403 | 404 | 405 | 409 | 410 | 411 => ServiceStatus::Finished,

            // 	The payload was too big.
            413 => {
                if self.batch_len == 1 {
                    // TODO: error
                    ServiceStatus::Finished
                } else {
                    self.batch_len %= 2;
                    ServiceStatus::Remaining
                }
            }

            // The request rate quota has been exceeded.
            429 => {
                let duration = res
                    .headers()
                    .get("retry-after")
                    .and_then(|val| val.to_str().ok())
                    .and_then(|val| val.parse::<u64>().ok())
                    .map(Duration::from_secs);

                match duration {
                    Some(duration) => ServiceStatus::Timeount(duration),
                    // TODO: error
                    _ => ServiceStatus::Finished,
                }
            }

            _ => {
                if self.retry_count == 0 {
                    self.retry_count += 1;
                    // retry immediately
                    ServiceStatus::Timeount(Duration::from_secs(0))
                } else if self.retry_count <= 5 {
                    self.retry_count += 1;
                    // retry after 2^n seconds
                    ServiceStatus::Timeount(Duration::from_secs(
                        2_u64.pow(self.retry_count - 1_u32),
                    ))
                } else {
                    // TODO: error
                    ServiceStatus::Finished
                }
            }
        }
    }
}

trait Sendable {
    fn build_request(data: &[Self], api: &Api) -> RequestBuilder
    where
        Self: Sized;
}

#[derive(Serialize)]
pub struct Logs {
    pub logs: Vec<NrLog>,
}

impl Sendable for Logs {
    fn build_request(data: &[Logs], api: &Api) -> RequestBuilder {
        let url = match &api.log_endpoint {
            ApiEndpoint::US => "https://log-api.newrelic.com/log/v1".into(),
            ApiEndpoint::EU => "https://log-api.eu.newrelic.com/log/v1".into(),
            ApiEndpoint::Custom(domain) => format!("{domain}/log/v1"),
        };
        // https://docs.newrelic.com/docs/logs/log-api/introduction-log-api/#json-headers
        api.client
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_ENCODING, "gzip")
            .header("Api-Key", &api.key)
            .body(to_gz(&data))
    }
}

#[derive(Serialize)]
pub struct Spans {
    pub spans: Vec<NrSpan>,
}

impl Sendable for Spans {
    fn build_request(data: &[Spans], api: &Api) -> RequestBuilder {
        let url = match &api.log_endpoint {
            ApiEndpoint::US => "https://trace-api.newrelic.com/trace/v1".into(),
            ApiEndpoint::EU => "https://trace-api.eu.newrelic.com/trace/v1".into(),
            ApiEndpoint::Custom(domain) => format!("{domain}/trace/v1"),
        };
        // https://docs.newrelic.com/docs/distributed-tracing/trace-api/trace-api-general-requirements-limits/#headers-query-parameters
        api.client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_ENCODING, "gzip")
            .header("Api-Key", &api.key)
            .header("Data-Format", "newrelic")
            .header("Data-Format-Version", "1")
            .body(to_gz(&data))
    }
}

#[inline]
fn to_gz<T: Serialize>(data: T) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    serde_json::to_writer(&mut encoder, &data).unwrap();
    encoder.finish().unwrap()
}
