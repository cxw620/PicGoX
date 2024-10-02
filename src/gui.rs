//! UI logic

pub(crate) mod tray;

use std::sync::OnceLock;

use slint::{ComponentHandle, PlatformError};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::task::{
    state::{UploadState, WindowState},
    State, TaskT,
};

mod generated {
    slint::include_modules!();
}

/// Global UI state updater channel
static UI_STATE_TX: OnceLock<Sender<State>> = OnceLock::new();

/// GUI Handler
pub(crate) struct Handler(generated::Main);

impl Handler {
    /// Initialize the handler
    pub(crate) fn init(runtime: &Runtime) -> Self {
        let ui_handler = generated::Main::new().unwrap_or_else(|e| {
            panic!("Failed to initialize GUI: {:#?}", e);
        });

        // Set interrupt callback.
        ui_handler.window().on_close_requested(|| {
            #[cfg(windows)]
            match popup_tips("Really quit?") {
                Some(r) => match r {
                    win_msgbox::OkayCancel::Okay => {
                        tracing::info!("User confirmed to quit.");

                        State::Window(WindowState::Hidden).blocking_send_page_state();

                        slint::CloseRequestResponse::HideWindow
                    }
                    win_msgbox::OkayCancel::Cancel => {
                        tracing::info!("User cancelled to quit.");
                        slint::CloseRequestResponse::KeepWindowShown
                    }
                },
                None => {
                    State::Window(WindowState::Hidden).blocking_send_page_state();
                    slint::CloseRequestResponse::HideWindow
                }
            }

            #[cfg(not(windows))]
            {
                State::Window(WindowState::Hidden)
                    .blocking_send_page_state()
                    .trigger_task();
                slint::CloseRequestResponse::HideWindow
            }
        });

        let (state_handler, tx) = StateHandler::init(&ui_handler);

        UI_STATE_TX
            .set(tx)
            .expect("Should initialize UI state updater channel only once!");

        runtime.spawn(state_handler.recv());

        Self(ui_handler).register_callback()
    }

    fn register_callback(self) -> Self {
        let handler = self.0;

        // Page 0: Upload page
        {
            let upload_page_adapter = handler.global::<generated::UploadPageAdapter>();

            upload_page_adapter.on_do_upload(|| {
                tracing::debug!("Do upload!");
                UploadState::Uploading { progress: 0.0 }
                    .blocking_send_page_state()
                    .trigger_task();
            });

            upload_page_adapter.on_cancel_upload(|| {
                tracing::debug!("Cancel upload!");
                UploadState::Cancelling.blocking_send_page_state().trigger_task();
            });
        }

        Self(handler)
    }

    /// Shortcut for running the event loop.
    ///
    /// Will block the current thread.
    pub(crate) fn run(self) -> Result<(), PlatformError> {
        self.0.show()?;
        slint::run_event_loop_until_quit()?;
        self.0.hide()?;

        Ok(())
    }
}

/// UI State handler
pub(crate) struct StateHandler {
    inner: slint::Weak<generated::Main>,

    rx: Receiver<State>,
}

impl StateHandler {
    /// Initialize the state handler
    pub(crate) fn init(handler: &generated::Main) -> (Self, Sender<State>) {
        let (tx, rx) = mpsc::channel(1);

        let handler_weak = handler.as_weak();

        (
            Self {
                inner: handler_weak,
                rx,
            },
            tx,
        )
    }

    /// Main loop
    pub(crate) async fn recv(mut self) {
        while let Some(state) = self.rx.recv().await {
            let ui_handle = self.inner.clone();
            let result = match state {
                State::Upload(state) => match state {
                    UploadState::Idle => slint::invoke_from_event_loop(move || {
                        ui_handle
                            .unwrap()
                            .global::<generated::UploadPageAdapter>()
                            .invoke_set_state_idle();
                    }),
                    UploadState::Uploading { progress } => slint::invoke_from_event_loop(move || {
                        ui_handle
                            .unwrap()
                            .global::<generated::UploadPageAdapter>()
                            .invoke_set_state_uploading(progress);
                    }),
                    UploadState::Cancelling => slint::invoke_from_event_loop(move || {
                        ui_handle
                            .unwrap()
                            .global::<generated::UploadPageAdapter>()
                            .invoke_set_state_cancelling();
                    }),
                },
                State::Window(state) => {
                    match state {
                        WindowState::Shown => {
                            let _ = slint::invoke_from_event_loop(move || {
                                let _ = ui_handle.unwrap().window().show();
                            });
                            let _ = tray::Tray::toggle_button();
                        }
                        WindowState::Hidden => {
                            let _ = slint::invoke_from_event_loop(move || {
                                let _ = ui_handle.unwrap().window().hide();
                            });
                            let _ = tray::Tray::toggle_button();
                        }
                        _ => {}
                    };

                    continue;
                }
            };

            if let Err(e) = result {
                tracing::error!("Failed to update UI state: {:#?}", e);
            }
        }

        tracing::debug!("UI state updater channel closed!")
    }
}

/// Page state trait
pub(crate) trait PageStateT {
    /// Send a state update to the UI page
    fn blocking_send_page_state(self) -> Self;

    #[allow(dead_code)]
    /// Send a state update to the UI page
    async fn send_page_state(self) -> Self;
}

impl<T> PageStateT for T
where
    T: Into<State> + Copy,
{
    #[inline]
    fn blocking_send_page_state(self) -> Self {
        let _ = UI_STATE_TX
            .get()
            .expect("UI state updater channel should be initialized!")
            .blocking_send(self.into());

        self
    }

    #[inline]
    async fn send_page_state(self) -> Self {
        let _ = UI_STATE_TX
            .get()
            .expect("UI state updater channel should be initialized!")
            .send(self.into())
            .await;

        self
    }
}

#[cfg(windows)]
pub(super) fn popup_tips(message: &str) -> Option<win_msgbox::OkayCancel> {
    win_msgbox::warning::<win_msgbox::OkayCancel>(message)
        .title("PicGoX")
        .show()
        .inspect_err(|e| {
            tracing::error!("Error {e} on creating Windows MessageBox.");
        })
        .ok()
}
