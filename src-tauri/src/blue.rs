use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use arctic::{
    v2::{EventHandler, EventType},
    HeartRate, PmdData, PmdRead,
};
use chrono::Utc;
use serde::Deserialize;
use tauri::{async_runtime::Mutex, PathResolver};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{self, AsyncWriteExt, BufWriter},
};

use crate::AppError;

/// [`EventHandler`] for the [`PolarSensor`]
pub struct Handler {
    /// datetime of start so cache files are unique between runs
    pub prefix: String,
    hr_writer: Arc<Mutex<BufWriter<File>>>,
    hr_count: AtomicUsize,
    acc_writer: Arc<Mutex<BufWriter<File>>>,
    ecg_writer: Arc<Mutex<BufWriter<File>>>,
}

impl Handler {
    pub async fn new(config: &Config, path_resolver: &PathResolver) -> Result<Self, AppError> {
        tracing::info!("creating handler");

        let prefix_path = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        tracing::info!(
            "saving data to {}",
            path_resolver.app_data_dir().unwrap().to_string_lossy(),
        );
        fs::create_dir_all(path_resolver.app_data_dir().unwrap()).await?;
        tracing::debug!("prefix is {prefix_path}");
        let hr_writer = {
            let path = path(
                path_resolver.app_data_dir().unwrap(),
                &prefix_path,
                EventType::Hr,
            );
            {
                let mut lock = crate::HR_PATH.lock().unwrap();
                *lock = Some(path.clone());
            }
            tracing::debug!("made path: {path:?}");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .await?;
            write_metadata(config, &mut file).await?;
            write_headers(EventType::Hr, &mut file).await?;
            // smaller buffer because there's more infrequent
            // data.
            let buf = BufWriter::with_capacity(256, file);
            Arc::new(Mutex::new(buf))
        };
        let acc_writer = {
            let path = path(
                path_resolver.app_data_dir().unwrap(),
                &prefix_path,
                EventType::Acc,
            );
            {
                let mut lock = crate::ACC_PATH.lock().unwrap();
                *lock = Some(path.clone());
            }
            tracing::debug!("made path: {path:?}");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .await?;
            write_metadata(config, &mut file).await?;
            write_headers(EventType::Acc, &mut file).await?;
            let buf = BufWriter::new(file);
            Arc::new(Mutex::new(buf))
        };
        let ecg_writer = {
            let path = path(
                path_resolver.app_data_dir().unwrap(),
                &prefix_path,
                EventType::Ecg,
            );
            {
                let mut lock = crate::ECG_PATH.lock().unwrap();
                *lock = Some(path.clone());
            }
            tracing::debug!("made path: {path:?}");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path)
                .await?;
            write_metadata(config, &mut file).await?;
            write_headers(EventType::Ecg, &mut file).await?;
            let buf = BufWriter::new(file);
            Arc::new(Mutex::new(buf))
        };
        let me = Self {
            prefix: prefix_path,
            hr_writer,
            hr_count: 0.into(),
            acc_writer,
            ecg_writer,
        };

        tracing::debug!("finished creating handler");

        Ok(me)
    }
}

fn path(base: PathBuf, prefix: &str, ty: EventType) -> PathBuf {
    match ty {
        EventType::Hr => base.join(format!("hr_{}.csv", prefix)),
        EventType::Acc => base.join(format!("acc_{}.csv", prefix)),
        EventType::Ecg => base.join(format!("ecg_{}.csv", prefix)),
        EventType::Battery => unreachable!(),
    }
}

#[arctic::async_trait]
impl EventHandler for Handler {
    async fn heart_rate_update(&self, heartrate: HeartRate) {
        tracing::trace!(bpm=heartrate.bpm(), rr=?heartrate.rr());
        let mut rr = String::new();
        for r in heartrate.rr().into_iter().flatten() {
            rr.push_str(format!(",{}", r).as_str());
        }
        let count = self.hr_count.fetch_add(1, Ordering::AcqRel);
        self.hr_writer
            .lock()
            .await
            .write_all(format!("{},{}{}\n", count, heartrate.bpm(), rr).as_bytes())
            .await
            .unwrap();
    }

    async fn measurement_update(&self, data: PmdRead) {
        tracing::trace!(?data);
        let timestamp = data.time_stamp();
        for point in data.data() {
            let (mut writer, message) = match point {
                PmdData::Acc(acc) => {
                    let (x, y, z) = acc.data();
                    (
                        self.acc_writer.lock().await,
                        format!("{},{},{},{}\n", timestamp, x, y, z),
                    )
                }
                PmdData::Ecg(ecg) => {
                    let val = ecg.val();
                    (
                        self.ecg_writer.lock().await,
                        format!("{},{}\n", timestamp, val),
                    )
                }
            };

            writer.write_all(message.as_bytes()).await.unwrap();
        }
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        tracing::info!("flushing handler writers");
        let hr_writer = Arc::clone(&self.hr_writer);
        let acc_writer = Arc::clone(&self.acc_writer);
        let ecg_writer = Arc::clone(&self.ecg_writer);

        tokio::task::spawn(async move {
            if let Err(e) = hr_writer.lock().await.flush().await {
                tracing::error!("couldn't flush hr writer: {e}");
            };
            if let Err(e) = acc_writer.lock().await.flush().await {
                tracing::error!("couldn't flush acc writer: {e}");
            };
            if let Err(e) = ecg_writer.lock().await.flush().await {
                tracing::error!("couldn't flush ecg writer: {e}");
            };
        });
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub participant_id: String,
    pub session_number: u64,
    pub trial_id: u64,
    pub description: String,
    pub range: u8,
    pub rate: u8,
}

async fn write_metadata(config: &Config, file: &mut File) -> Result<(), io::Error> {
    let metadata = format!(
        "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
        config.participant_id,
        config.session_number,
        config.trial_id,
        Utc::now(),
        config.description
    );

    file.write_all(metadata.as_bytes()).await?;

    Ok(())
}

async fn write_headers(ty: EventType, file: &mut File) -> Result<(), io::Error> {
    let header = match ty {
        EventType::Hr => "time,bpm,rr\n",
        EventType::Acc => "time,x,y,z\n",
        EventType::Ecg => "time,val\n",
        EventType::Battery => unreachable!(),
    };

    file.write_all(header.as_bytes()).await?;

    Ok(())
}
