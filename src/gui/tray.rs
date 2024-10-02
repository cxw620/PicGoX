//! System tray

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};

use tray_item::TrayItem;

use crate::{
    gui::PageStateT,
    task::{state::WindowState, State, TaskT},
};

static TRAY_ITEM: OnceLock<Mutex<TrayItem>> = OnceLock::new();
static ID: OnceLock<u32> = OnceLock::new();
static STATE: AtomicBool = AtomicBool::new(true);

const STATE_SHOWN_HINT: &str = "退出至托盘";
const STATE_HIDDEN_HINT: &str = "显示窗口";

#[allow(dead_code)]
pub(crate) struct Tray;

impl Tray {
    /// Initialize the tray item
    pub(crate) fn init() -> Result<(), tray_item::TIError> {
        let mut item = tray_item::TrayItem::new("PicGoX", tray_item::IconSource::Resource("tray-default"))?;

        // title
        item.add_label("PicGoX - 截图上传工具")?;

        // menu items

        // Button: To show or hide the window
        item.inner_mut().add_separator()?;
        let button_id = item.inner_mut().add_menu_item_with_id(STATE_SHOWN_HINT, || {
            let current_state = STATE.load(Ordering::Acquire);
            tracing::debug!("[TRAY] Clicked: Button: To show or hide the window: {}", !current_state);
            if current_state {
                State::Window(WindowState::Hidden).blocking_send_page_state();
            } else {
                State::Window(WindowState::Shown).blocking_send_page_state();
            }
        })?;
        ID.set(button_id)
            .unwrap_or_else(|_| panic!("Should initialize tray item only once!"));

        // Button: To close the window
        item.inner_mut().add_menu_item_with_id("退出", || {
            tracing::debug!("[TRAY] Clicked: Button: To close the window and exit");

            if super::popup_tips("Really quit?").is_some_and(|ret| ret == win_msgbox::OkayCancel::Okay) {
                tracing::debug!("[TRAY] User confirmed to close the window.");
                State::Window(WindowState::Closed).trigger_task();
            } else {
                tracing::debug!("[TRAY] User cancelled to close the window.");
            }
        })?;

        TRAY_ITEM
            .set(Mutex::new(item))
            .unwrap_or_else(|_| panic!("Should initialize tray item only once!"));

        Ok(())
    }

    pub(crate) fn toggle_button() -> Result<(), tray_item::TIError> {
        let mut item = TRAY_ITEM
            .get()
            .expect("Tray item should be initialized!")
            .lock()
            .unwrap();
        let button_id = *ID.get().expect("Tray item should be initialized!");
        let current_state = STATE.load(Ordering::Acquire);

        let target_state = !current_state;

        tracing::debug!("Toggle tray button state from {current_state} to {target_state}");

        if target_state {
            item.inner_mut().set_menu_item_label(STATE_SHOWN_HINT, button_id)?;
        } else {
            item.inner_mut().set_menu_item_label(STATE_HIDDEN_HINT, button_id)?;
        }
        STATE.store(target_state, Ordering::Release);

        Ok(())
    }
}
