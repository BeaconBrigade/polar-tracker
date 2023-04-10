#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod blue;

use std::{path::PathBuf, time::Duration};

use arctic::{Error, H10MeasurementType, NotifyStream, PolarSensor};
use serde::Serialize;
use tauri::State;
use thiserror::Error;
use tokio::sync::{
    watch::{channel, Sender},
    Mutex,
};

use crate::blue::{Config, Handler};

fn main() {
    tracing_subscriber::fmt().without_time().init();
    tauri::Builder::default()
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![set_config, connect, disconnect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AppState {
    pub stop_tx: Sender<bool>,
    pub config: Option<Config>,
    pub sensor: Option<PolarSensor>,
    pub handler: Option<Handler>,
    pub hr_file: Option<PathBuf>,
    pub acc_file: Option<PathBuf>,
    pub ecg_file: Option<PathBuf>,
}

impl AppState {
    fn new() -> Self {
        let (tx, rx) = channel(true);
        Self {
            stop_tx: tx,
            config: None,
            sensor: None,
            handler: Some(Handler::new(rx)),
            hr_file: None,
            acc_file: None,
            ecg_file: None,
        }
    }
}

#[tauri::command]
async fn set_config(state: State<'_, Mutex<AppState>>, config: Config) -> Result<(), AppError> {
    let mut lock = state.lock().await;
    lock.config = Some(config);

    Ok(())
}

#[tauri::command]
async fn connect(state: State<'_, Mutex<AppState>>, device_id: String) -> Result<(), AppError> {
    let mut lock = state.lock().await;
    let Some(config) = &lock.config else {
        return Err(AppError::MissingConfig);
    };

    let mut sensor = PolarSensor::new(device_id).await?;

    while !sensor.is_connected().await {
        match sensor.connect().await {
            Err(e @ Error::NoBleAdaptor) => {
                tracing::error!("no bluetooth adaptor found");
                Err(e)?
            }
            Err(e) => tracing::warn!("could not connect: {e}"),
            Ok(_) => {}
        }
    }

    // polar-arctic is the dumbest library the other person should not have
    // let me push to 1.0 literally every part I touched is super weird and unergonomic
    let _ = sensor.range(config.range);
    let _ = sensor.sample_rate(config.rate);

    if config.measure_hr {
        sensor.subscribe(NotifyStream::HeartRate).await?;
    }
    if config.measure_acc || config.measure_ecg {
        sensor.subscribe(NotifyStream::MeasurementData).await?;
    }

    // once again, more stupid
    if config.measure_acc {
        sensor.data_type_push(H10MeasurementType::Acc);
    }
    if config.measure_ecg {
        sensor.data_type_push(H10MeasurementType::Ecg);
    }

    sensor.event_handler(lock.handler.take().ok_or(AppError::MissingHandler)?);

    lock.sensor = Some(sensor);

    Ok(())
}

#[tauri::command]
async fn disconnect(state: State<'_, Mutex<AppState>>) -> Result<(), AppError> {
    let mut lock = state.lock().await;

    // stop old sensor and drop it
    lock.stop_tx.send(false).unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let _ = lock.sensor.take();

    // create new handler and channel
    let (tx, rx) = channel(true);
    lock.handler = Some(Handler::new(rx));
    lock.stop_tx = tx;

    Ok(())
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    ArcticError(#[from] arctic::Error),
    #[error("no config found")]
    MissingConfig,
    #[error("tried to connect to device without a handler")]
    MissingHandler,
    #[error("sensor was absent when it was needed")]
    MissingSensor,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = match self {
            Self::ArcticError(v) => v.to_string(),
            Self::MissingConfig => "no config found".to_string(),
            Self::MissingHandler => "tried to connect to device without a handler".to_string(),
            Self::MissingSensor => "sensor was absent when it was needed".to_string(),
        };

        serializer.serialize_str(&str)
    }
}
