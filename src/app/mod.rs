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

    // 创建封面图标（右侧）
    let cover_image = gtk::Image::builder()
        .pixel_size(32)
        .margin_start(8)
        .name("cover-image")
        .build();
    
    // 设置默认灰色背景
    cover_image.set_icon_name(Some("image-missing"));

    // 创建水平 Box 包含图标和 label
    let lyric_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .name("lyric-container")
        .build();

    lyric_box.append(&app_icon);
    lyric_box.append(&lyric_label);
    lyric_box.append(&cover_image);

    // 保存封面图标的引用
    window.imp().cover_image.set(cover_image).ok();

    let verical_box = gtk::Box::builder()
        .baseline_position(gtk::BaselinePosition::Center)
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Start)
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

pub fn set_cover_image(window: &Window, art_url: Option<&str>) {
    use gtk::gdk::Texture;
    use gtk::gdk_pixbuf::Pixbuf;
    
    let Some(cover_image) = window.imp().cover_image.get() else {
        return;
    };

    match art_url {
        Some(url) if url.starts_with("file://") => {
            // 本地文件 - 直接让 GTK 加载和缩放
            if let Some(path) = url::Url::parse(url).ok().and_then(|u| u.to_file_path().ok()) {
                if let Ok(pixbuf) = Pixbuf::from_file(&path) {
                    // 根据图片尺寸计算圆角半径（假设最终显示为32px，圆角8px，则比例为8/32=0.25）
                    let min_dimension = pixbuf.width().min(pixbuf.height()) as f64;
                    let radius = min_dimension * 0.25; // 圆角占图片边长的25%
                    let rounded_pixbuf = apply_rounded_corners(&pixbuf, radius);
                    let texture = Texture::for_pixbuf(&rounded_pixbuf);
                    cover_image.set_paintable(Some(&texture));
                    return;
                }
            }
            // 加载失败，显示灰色占位图
            cover_image.set_icon_name(Some("image-missing"));
        }
        Some(url) if url.starts_with("http://") || url.starts_with("https://") => {
            // 网络图片 - 异步加载
            let cover_image_weak = cover_image.downgrade();
            let url = url.to_string();
            crate::glib_spawn!(async move {
                match load_image_from_url(&url).await {
                    Ok(texture) => {
                        if let Some(cover_image) = cover_image_weak.upgrade() {
                            cover_image.set_paintable(Some(&texture));
                        }
                    }
                    Err(_) => {
                        if let Some(cover_image) = cover_image_weak.upgrade() {
                            cover_image.set_icon_name(Some("image-missing"));
                        }
                    }
                }
            });
        }
        _ => {
            // 没有封面或不支持的格式，显示灰色占位图
            cover_image.set_icon_name(Some("image-missing"));
        }
    }
}

fn apply_rounded_corners(pixbuf: &gtk::gdk_pixbuf::Pixbuf, radius: f64) -> gtk::gdk_pixbuf::Pixbuf {
    use gtk::cairo;

    let width = pixbuf.width();
    let height = pixbuf.height();

    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
        .expect("Failed to create surface");

    {
        let cr = cairo::Context::new(&surface).expect("Failed to create context");

        // 创建圆角路径
        let x = 0.0;
        let y = 0.0;
        let w = width as f64;
        let h = height as f64;

        cr.new_sub_path();
        cr.arc(x + w - radius, y + radius, radius, -std::f64::consts::PI / 2.0, 0.0);
        cr.arc(x + w - radius, y + h - radius, radius, 0.0, std::f64::consts::PI / 2.0);
        cr.arc(x + radius, y + h - radius, radius, std::f64::consts::PI / 2.0, std::f64::consts::PI);
        cr.arc(x + radius, y + radius, radius, std::f64::consts::PI, 3.0 * std::f64::consts::PI / 2.0);
        cr.close_path();

        cr.clip();

        // 绘制原始图片
        cr.set_source_pixbuf(pixbuf, 0.0, 0.0);
        cr.paint().expect("Failed to paint");
    } // Context 在这里被释放

    // 使用 GTK 提供的函数从 Cairo Surface 创建 Pixbuf
    // 这会自动处理颜色空间转换和 alpha 反预乘
    gtk::gdk::pixbuf_get_from_surface(&surface, 0, 0, width, height)
        .expect("Failed to create pixbuf from surface")
}

async fn load_image_from_url(url: &str) -> anyhow::Result<gtk::gdk::Texture> {
    use gtk::glib;
    
    // 在tokio运行时中下载图片
    let url_owned = url.to_string();
    let bytes = crate::TOKIO_RUNTIME.spawn(async move {
        let response = reqwest::get(&url_owned).await?;
        response.bytes().await.map_err(|e| anyhow::anyhow!("{}", e))
    }).await??;

    // 在GLib主线程中创建Pixbuf（加载原图，不缩放）
    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_stream(
        &gtk::gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&bytes)),
        gtk::gio::Cancellable::NONE,
    )?;

    // 根据图片尺寸计算圆角半径
    let min_dimension = pixbuf.width().min(pixbuf.height()) as f64;
    let radius = min_dimension * 0.25; // 圆角占图片边长的25%
    let rounded_pixbuf = apply_rounded_corners(&pixbuf, radius);

    // 创建 Texture，让 GTK 自己处理缩放
    let texture = gtk::gdk::Texture::for_pixbuf(&rounded_pixbuf);

    Ok(texture)
}
