//! Async task handler

pub(crate) mod state;

use std::sync::OnceLock;

use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::gui::PageStateT;

/// Global UI state updater channel
static STATE_TX: OnceLock<Sender<State>> = OnceLock::new();

pub(crate) struct TaskHandler {
    rx: Receiver<State>,
}

impl TaskHandler {
    /// Initialize the task handler
    pub(crate) fn init() -> Self {
        let (tx, rx) = mpsc::channel(256);

        STATE_TX
            .set(tx)
            .expect("Should initialize UI state updater channel only once!");

        Self { rx }
    }

    /// Spawn the task handler for UI state updates
    pub(crate) fn spawn(mut self, runtime: &Runtime) {
        runtime.spawn(async move {
            let mut current_upload_task = None;

            while let Some(state) = self.rx.recv().await {
                match state {
                    State::Upload(state) => match state {
                        state::UploadState::Idle => {
                            tracing::debug!("Upload state: idle");
                        }
                        state::UploadState::Uploading { .. } => {
                            tracing::debug!("Upload state: uploading");

                            let handle = tokio::spawn(async {
                                let mut progress = 0.0;

                                while progress < 1.0 {
                                    progress += 0.01;

                                    state::UploadState::Uploading { progress }.send_page_state().await;

                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }

                                tracing::debug!("Upload state: finished");
                                state::UploadState::Idle.send_page_state().await;
                            });

                            current_upload_task = Some(handle);
                        }
                        state::UploadState::Cancelling => {
                            tracing::debug!("Upload state: cancelling");

                            if let Some(task) = current_upload_task.take() {
                                task.abort();
                                let _ = task.await;
                            } else {
                                tracing::warn!("Upload state: cancelled but no task running!");
                            }

                            tracing::debug!("Upload state: cancelled");
                            state::UploadState::Idle.send_page_state().await;
                        }
                    },
                    State::Window(state) => match state {
                        state::WindowState::Shown => {
                            tracing::info!("Window state is to be: shown")
                        }
                        state::WindowState::Hidden => {
                            tracing::info!("Window state is to be: hidden")
                        }
                        state::WindowState::Closed => {
                            tracing::info!("Window state is to be: closed");
                            if let Err(e) = slint::quit_event_loop() {
                                tracing::error!("Failed to quit event loop: {:#?}", e);
                            }
                        }
                    },
                }
            }

            tracing::debug!("Task handler closed!")
        });
    }
}

/// Task that binds to specific state
pub(crate) trait TaskT {
    /// Trigger the task
    fn trigger_task(self);
}

impl<T> TaskT for T
where
    T: Into<State> + Copy,
{
    fn trigger_task(self) {
        let _ = STATE_TX
            .get()
            .expect("UI state updater channel should be initialized!")
            .blocking_send(self.into());
    }
}

#[derive(Debug, Clone, Copy)]
/// Page State
pub(crate) enum State {
    /// Page 0: Upload page
    Upload(state::UploadState),

    /// Window: state
    Window(state::WindowState),
}

impl From<state::UploadState> for State {
    fn from(state: state::UploadState) -> Self {
        Self::Upload(state)
    }
}
