use std::time::{UNIX_EPOCH, SystemTime};

use arctic::{EventHandler, HeartRate, PmdRead, PolarSensor};
use serde::Deserialize;
use tokio::sync::watch::Receiver;

/// [`EventHandler`] for the [`PolarSensor`]
pub struct Handler {
    /// receiver to tell arctic when to stop
    pub stop_rx: Receiver<bool>,
    /// datetime of start so cache files are unique between runs
    pub prefix_path: String,
}

impl Handler {
    pub fn new(stop_rx: Receiver<bool>) -> Self {
        let prefix_path = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        Self {
            stop_rx,
            prefix_path,
        }
    }
}

#[arctic::async_trait]
impl EventHandler for Handler {
    async fn heart_rate_update(&self, _ctx: &PolarSensor, heartrate: HeartRate) {
        tracing::debug!(bpm=heartrate.bpm(), rr=?heartrate.rr());
    }

    async fn measurement_update(&self, _ctx: &PolarSensor, data: PmdRead) {
        tracing::debug!(?data);
    }

    async fn should_continue(&self) -> bool {
        *self.stop_rx.borrow()
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub participant_id: String,
    pub session_number: u64,
    pub trial_id: u64,
    pub description: String,
    pub measure_hr: bool,
    pub measure_acc: bool,
    pub measure_ecg: bool,
    pub range: u8,
    pub rate: u8,
}
