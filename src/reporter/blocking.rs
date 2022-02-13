use std::thread::{self, JoinHandle};
use tokio::runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use crate::api::{Api, Logs, Spans};
use crate::types::{NrLog, NrSpan};

use super::Reporter;

/// A [`Reporter`] using `newrelic_telemetry::blocking::Client`
///
/// [`Reporter`]: ../Reporter
pub struct BlockingReporter {
    channel: Option<UnboundedSender<(Vec<NrLog>, Vec<NrSpan>)>>,
    handle: Option<JoinHandle<()>>,
}

impl BlockingReporter {
    pub fn new(api: Api) -> BlockingReporter {
        let (tx, mut rx) = unbounded_channel::<(Vec<NrLog>, Vec<NrSpan>)>();

        let handle = thread::Builder::new()
            .name("newrelic-report".into())
            .spawn(move || {
                let rt = match runtime::Builder::new_current_thread().enable_all().build() {
                    Err(e) => {
                        eprintln!("Failed to communicate runtime creation failure: {:?}", e);
                        return;
                    }
                    Ok(v) => v,
                };

                rt.block_on(async move {
                    while let Some((logs, spans)) = rx.recv().await {
                        api.send(vec![Logs { logs }], vec![Spans { spans }]).await
                    }
                });

                drop(rt);
            })
            .expect("failed to spawn thread");

        BlockingReporter {
            channel: Some(tx),
            handle: Some(handle),
        }
    }
}

impl Reporter for BlockingReporter {
    fn report(&self, spans: Vec<NrSpan>, logs: Vec<NrLog>) {
        if let Some(channel) = &self.channel {
            let _ = channel.send((logs, spans));
        }
    }
}

impl Drop for BlockingReporter {
    fn drop(&mut self) {
        if let Some(channel) = self.channel.take() {
            drop(channel);
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
