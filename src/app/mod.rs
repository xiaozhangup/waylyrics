pub mod search_window;
mod window;

use gtk::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{Application, Label};
pub use window::Window;

use crate::app::utils::set_click_pass_through;
use crate::{config, DEFAULT_TEXT};

const WINDOW_MIN_HEIGHT: i32 = 120;

pub mod actions;
pub mod dialog;
pub mod utils;

pub fn build_main_window(
    app: &Application,
    enable_filter_regex: bool,
    cache_lyrics: bool,
    length_toleration_ms: u128,
    show_default_text_on_idle: bool,
    show_lyric_on_pause: bool,
    primary_font_size: u32,
    secondary_font_size: u32,
) -> Window {
    let window = Window::new(
        app,
        cache_lyrics,
        length_toleration_ms,
        show_default_text_on_idle,
        show_lyric_on_pause,
        primary_font_size,
        secondary_font_size,
    );

    window.set_size_request(500, WINDOW_MIN_HEIGHT);
    window.set_title(Some(DEFAULT_TEXT));
    window.set_icon_name(Some(crate::APP_ID_FIXED));
    window.present();

    // 创建应用图标
    let app_icon = gtk::Image::builder()
        .icon_name(crate::APP_ID_FIXED)
        .pixel_size(32)
        .margin_end(8)
        .name("lyric-icon")
        .build();

    // 创建歌词 label
    let lyric_label = Label::builder()
        .label("Waylyrics")
        .name("lyric")
        .hexpand(true)
        .use_markup(true)
        .justify(gtk::Justification::Center)
        .build();

    utils::setup_label(&lyric_label, enable_filter_regex);

    // 创建水平 Box 包含图标和 label
    let lyric_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .name("lyric-container")
        .build();

    lyric_box.append(&app_icon);
    lyric_box.append(&lyric_label);

    let verical_box = gtk::Box::builder()
        .baseline_position(gtk::BaselinePosition::Center)
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();

    verical_box.insert_child_after(&lyric_box, gtk::Box::NONE);

    window.set_child(Some(&verical_box));

    let align = window.imp().lyric_align.get();
    set_lyric_align(&window, align);

    window.connect_decorated_notify(|window| {
        crate::log::debug!("triggered decorated signal");
        let clickthrough = window.imp().clickthrough.get();
        set_click_pass_through(window, clickthrough)
    });

    window.set_icon_name(Some(crate::APP_ID_FIXED));

    window
}

pub fn set_lyric_align(window: &Window, align: config::Align) -> Option<()> {
    let vbox: gtk::Box = window.child()?.downcast().ok()?;
    let lyric_box: gtk::Box = vbox.first_child()?.downcast().ok()?;
    lyric_box.set_halign(align.into());
    window.imp().lyric_align.set(align);
    Some(())
}

pub fn get_label(window: &Window) -> Option<Label> {
    let vbox: gtk::Box = window.child()?.downcast().ok()?;
    let lyric_box: gtk::Box = vbox.first_child()?.downcast().ok()?;
    // 跳过第一个子元素（图标），获取第二个子元素（label）
    let lyric_label: Label = lyric_box.first_child()?.next_sibling()?.downcast().ok()?;
    Some(lyric_label)
}
