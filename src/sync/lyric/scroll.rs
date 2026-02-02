use std::time::Duration;

use gtk::glib::{self, ControlFlow, Priority, WeakRef};
use gtk::subclass::prelude::ObjectSubclassIsExt;

use crate::app::{self, get_label};
use crate::config::LyricDisplayMode;
use crate::log::*;
use crate::lyric_providers::{LyricLineOwned, LyricOwned};

use crate::sync::{LyricState, TrackState, LYRIC, TRACK_PLAYING_STATE};
use crate::utils::reset_lyric_labels;

pub fn register_lyric_display(app: WeakRef<app::Window>, interval: Duration) {
    glib::timeout_add_local_full(interval, Priority::HIGH, move || {
        let Some(window) = app.upgrade() else {
            return ControlFlow::Break;
        };

        let (paused, metainfo_not_found) = TRACK_PLAYING_STATE.with_borrow(
            |TrackState {
                 metainfo, paused, ..
             }| (*paused, metainfo.is_none()),
        );

        if metainfo_not_found {
            return ControlFlow::Continue;
        }

        trace!("refresh lyric with paused = {paused}");
        refresh_lyric(&window, paused);

        ControlFlow::Continue
    });
}

fn set_lyric_with_mode(
    window: &app::Window,
    translation: Option<&LyricLineOwned>,
    origin: Option<&LyricLineOwned>,
) {
    let (primary_text, secondary_text) = match window.imp().lyric_display_mode.get() {
        LyricDisplayMode::ShowBoth => {
            // 上方显示翻译（如果有），下方显示原文（如果有翻译的话）
            (translation.or(origin), translation.and(origin))
        }
        LyricDisplayMode::ShowBothRev => {
            // 上方显示原文，下方显示翻译
            (origin, translation)
        }
        LyricDisplayMode::Origin => {
            // 只显示原文
            (origin, None)
        }
        LyricDisplayMode::PreferTranslation => {
            // 只显示翻译（如果有的话，否则显示原文）
            (translation.or(origin), None)
        }
    };

    set_lyric(window, primary_text, secondary_text);
}

fn set_lyric(
    window: &app::Window,
    primary: Option<&LyricLineOwned>,
    secondary: Option<&LyricLineOwned>,
) {
    let primary_text = primary
        .map(|LyricLineOwned { text, .. }| text.as_str().trim())
        .unwrap_or_default();

    // 从配置中读取字体大小
    let primary_font_size = window.imp().primary_font_size.get();
    let secondary_font_size = window.imp().secondary_font_size.get();

    let label_text = if let Some(secondary_lyric) = secondary {
        let secondary_text = secondary_lyric.text.as_str().trim();
        if secondary_text.is_empty() {
            glib::markup_escape_text(primary_text).to_string()
        } else {
            // 使用markup格式，primary_text用大字号，secondary_text用小字号
            format!(
                "<span size=\"{}pt\">{}</span>\n<span size=\"{}pt\">{}</span>",
                primary_font_size,
                glib::markup_escape_text(primary_text),
                secondary_font_size,
                glib::markup_escape_text(secondary_text)
            )
        }
    } else {
        glib::markup_escape_text(primary_text).to_string()
    };

    if let Some(label) = get_label(window) {
        label.set_markup(&label_text);
        label.set_justify(gtk::Justification::Center);
    }
}

pub fn refresh_lyric(window: &app::Window, paused: bool) {
    if paused {
        if !window.imp().show_lyric_on_pause.get() {
            reset_lyric_labels(window, Some(""));
        }
        return;
    }

    LYRIC.with_borrow(
        |LyricState {
             origin,
             translation,
         }| {
            let system_time = window.imp().lyric_start.get().unwrap();
            let elapsed = system_time.elapsed().ok();
            let Some(elapsed) = elapsed else {
                return;
            };
            match (origin, translation) {
                (
                    LyricOwned::LineTimestamp(origin_lyric),
                    LyricOwned::LineTimestamp(translation_lyric),
                ) => {
                    let translation =
                        crate::lyric_providers::utils::find_next_lyric(&elapsed, translation_lyric);
                    let origin =
                        crate::lyric_providers::utils::find_next_lyric(&elapsed, origin_lyric);
                    set_lyric_with_mode(window, translation, origin);
                }
                (LyricOwned::LineTimestamp(origin_lyric), _) => {
                    let origin =
                        crate::lyric_providers::utils::find_next_lyric(&elapsed, origin_lyric);
                    set_lyric_with_mode(window, None, origin);
                }
                _ => (),
            }
        },
    );
}
