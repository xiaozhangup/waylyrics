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

    let lyric_label = Label::builder()
        .label("Waylyrics")
        .name("lyric")
        .vexpand(true)
        .use_markup(true)
        .justify(gtk::Justification::Center)
        .build();

    utils::setup_label(&lyric_label, enable_filter_regex);

    let verical_box = gtk::Box::builder()
        .baseline_position(gtk::BaselinePosition::Center)
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();

    verical_box.insert_child_after(&lyric_label, gtk::Box::NONE);

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
    let label = get_label(window)?;
    label.set_halign(align.into());
    window.imp().lyric_align.set(align);
    Some(())
}

pub fn get_label(window: &Window) -> Option<Label> {
    let vbox: gtk::Box = window.child()?.downcast().ok()?;
    let lyric_label: Label = vbox.first_child()?.downcast().ok()?;
    Some(lyric_label)
}
