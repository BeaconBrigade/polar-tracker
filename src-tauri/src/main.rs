#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod blue;

use std::{
    env,
    fs::OpenOptions,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use arctic::v2::{EventLoop, EventType, PolarHandle, PolarSensor};
use serde::Serialize;
use tauri::{api::dialog, CustomMenuItem, Manager, Menu, MenuItem, State, Submenu};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::instrument;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::blue::{Config, Handler};

pub static HR_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
pub static ACC_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
pub static ECG_PATH: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
pub static PREFIX: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn main() {
    let file_menu = Submenu::new(
        "File",
        Menu::new()
            .add_submenu(Submenu::new(
                "Export",
                Menu::new()
                    .add_item(CustomMenuItem::new("hr".to_string(), "Heart Rate Data"))
                    .add_item(CustomMenuItem::new("acc".to_string(), "Acceleration Data"))
                    .add_item(CustomMenuItem::new("ecg".to_string(), "ECG Data")),
            ))
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Quit),
    );
    let menu = Menu::new().add_submenu(file_menu);
    tauri::Builder::default()
        .menu(menu)
        .manage(AppState::new())
        .on_menu_event(|e| match e.menu_item_id() {
            name @ ("acc" | "ecg" | "hr") => {
                let name = name.to_string();
                let file_name = {
                    let app = e.window().app_handle();
                    let state: State<'_, AppState> = app.state();
                    let config = futures::executor::block_on(state.config.read());
                    let Some(config) = config.as_ref() else { return; };
                    let mut prefix = PREFIX.lock().unwrap();
                    let prefix = prefix.get_or_insert_with(|| {
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                            .to_string()
                    });

                    format!(
                        "{}_{}_{}_{}_{}.csv",
                        config.participant_id, config.session_number, config.trial_id, name, prefix,
                    )
                };
                dialog::FileDialogBuilder::default()
                    .set_file_name(&file_name)
                    .save_file(move |p| {
                        let Some(dest) = p else {
                            return;
                        };
                        tracing::info!("saving {name} to {}", dest.to_string_lossy());
                        let src_path = match name.as_str() {
                            "acc" => {
                                let Some(p) = ACC_PATH.lock().unwrap().clone() else {
                                    return;
                                };
                                p
                            }
                            "ecg" => {
                                let Some(p) = ECG_PATH.lock().unwrap().clone() else {
                                    return;
                                };
                                p
                            }
                            "hr" => {
                                let Some(p) = HR_PATH.lock().unwrap().clone() else {
                                    return;
                                };
                                p
                            }
                            _ => unreachable!(),
                        };

                        if let Err(e) = std::fs::copy(src_path, dest) {
                            tracing::error!("couldn't copy {name} file: {e}");
                        }
                    })
            }
            _ => {}
        })
        .setup(|app| {
            if env::args()
                .nth(1)
                .map(|a| a != "--print-logs")
                .unwrap_or(true)
            {
                let log_dir = app.path_resolver().app_log_dir().unwrap();
                let _ = std::fs::create_dir_all(&log_dir);

                let log_path = log_dir.join("polar-tracker.log");
                tracing_subscriber::registry()
                    .with(
                        EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| "app=INFO,arctic=INFO".into()),
                    )
                    .with(
                        fmt::layer().with_writer(
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(log_path)?,
                        ),
                    )
                    .init()
            } else {
                tracing_subscriber::registry()
                    .with(fmt::layer())
                    .with(
                        EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| "app=INFO,arctic=INFO".into()),
                    )
                    .init()
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_config,
            connect,
            disconnect,
            stop_event_loop,
            start_event_loop,
            cancel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AppState {
    pub config: RwLock<Option<Config>>,
    pub sensor: RwLock<Option<PolarSensor<EventLoop>>>,
    pub handle: RwLock<Option<PolarHandle>>,
    pub connecting: AtomicBool,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: None.into(),
            sensor: None.into(),
            handle: None.into(),
            connecting: false.into(),
        }
    }
}

#[tauri::command]
#[instrument(skip_all)]
async fn set_config(state: State<'_, AppState>, config: Config) -> Result<(), AppError> {
    tracing::info!("setting config: {config:?}");
    let mut lock = state.config.write().await;
    *lock = Some(config);

    Ok(())
}

#[tauri::command]
#[instrument(skip(state))]
async fn connect(state: State<'_, AppState>, device_id: String) -> Result<(), AppError> {
    tracing::info!("connecting to device");
    let config = state.config.read().await;
    let Some(config) = config.as_ref() else {
        tracing::error!("trying to connect without config");
        return Err(AppError::MissingConfig);
    };
    let range = config.range;
    let rate = config.rate;
    state.connecting.store(true, Ordering::SeqCst);

    let sensor = PolarSensor::new()
        .await?
        .map_connect(&device_id, |r| {
            match r {
                Err(e @ arctic::Error::NoBleAdaptor) => {
                    tracing::error!("no bluetooth adaptors found");
                    return Err(e);
                }
                Err(e) => {
                    tracing::warn!("could not connect: {}", e);
                    if !state.connecting.load(Ordering::Acquire) {
                        tracing::info!("cancel signal received");
                        return Err(e);
                    }
                }
                Ok(()) => {}
            };
            Ok(())
        })
        .await?
        .listen(EventType::Hr)
        .listen(EventType::Ecg)
        .listen(EventType::Acc)
        .listen(EventType::Battery)
        .range(range)
        .sample_rate(rate)
        .build()
        .await?;

    state.connecting.store(false, Ordering::Release);
    let mut lock = state.sensor.write().await;
    *lock = Some(sensor);

    Ok(())
}

#[tauri::command]
#[instrument(skip_all)]
async fn disconnect(state: State<'_, AppState>) -> Result<(), AppError> {
    tracing::info!("disconnecting from device");
    let mut lock = state.sensor.write().await;
    *lock = None;

    Ok(())
}

#[tauri::command]
#[instrument(skip_all)]
async fn stop_event_loop(state: State<'_, AppState>) -> Result<(), AppError> {
    tracing::info!("stopping measurement");
    let Some(handle) = state.handle.write().await.take() else {
        return Err(AppError::MissingHandler);
    };
    handle.stop().await;

    Ok(())
}

#[tauri::command]
#[instrument(skip_all)]
async fn start_event_loop(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    tracing::info!("starting event loop");
    let Some(sensor) = state.sensor.write().await.take() else {
        tracing::error!("started event loop without sensor");
        return Err(AppError::MissingSensor);
    };
    let config = state.config.read().await;
    let Some(config) = config.as_ref() else {
        tracing::error!("started event loop without config");
        return Err(AppError::MissingConfig);
    };

    let handler = Handler::new(config, &app.path_resolver()).await?;
    let handle = sensor.event_loop(handler).await;
    let mut lock = state.handle.write().await;
    *lock = Some(handle);

    Ok(())
}

#[tauri::command]
#[instrument(skip_all)]
async fn cancel(state: State<'_, AppState>) -> Result<(), AppError> {
    tracing::info!("canceling connection");
    state.connecting.store(false, Ordering::Release);
    Ok(())
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("arctic: {0}")]
    ArcticError(#[from] arctic::Error),
    #[error("fs: {0}")]
    IoError(#[from] tokio::io::Error),
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
        serializer.serialize_str(&self.to_string())
    }
}
