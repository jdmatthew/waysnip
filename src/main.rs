//! Waysnip - A Wayland screenshot selection tool

mod canvas;
mod clipboard;
mod screenshot;
mod selection;
mod window;

use canvas::Canvas;
use gtk4::gdk;
use gtk4::gio::ApplicationFlags;
use gtk4::glib;
use gtk4::prelude::*;
use screenshot::Screenshot;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const APP_ID: &str = "com.waysnip.Waysnip";

/// Result type for screenshot operations that can be displayed in UI
type ScreenshotResult<T> = Result<T, String>;

/// Generate a unique screenshot path in $HOME/Pictures
/// Format: screenshot-YYYY-MM-DD-HH-MM-SS.png
/// Adds -1, -2, etc. if file exists
fn generate_screenshot_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let pictures_dir = PathBuf::from(home).join("Pictures");

    // Ensure Pictures directory exists
    if !pictures_dir.exists() {
        std::fs::create_dir_all(&pictures_dir).ok()?;
    }

    // Get current timestamp with seconds
    let now = chrono::Local::now();
    let base_name = now.format("screenshot-%Y-%m-%d-%H-%M-%S").to_string();

    // Try the base name first
    let mut path = pictures_dir.join(format!("{}.png", base_name));
    if !path.exists() {
        return Some(path);
    }

    // If exists, add incrementing number
    for i in 1..1000 {
        path = pictures_dir.join(format!("{}-{}.png", base_name, i));
        if !path.exists() {
            return Some(path);
        }
    }

    // Fallback with milliseconds if somehow all are taken
    let name_with_ms = now.format("screenshot-%Y-%m-%d-%H-%M-%S-%3f").to_string();
    Some(pictures_dir.join(format!("{}.png", name_with_ms)))
}

/// Crop and get PNG data from canvas selection
fn get_cropped_png(canvas: &Canvas, screenshot: &Screenshot) -> ScreenshotResult<Vec<u8>> {
    let (x, y, w, h) = canvas
        .get_crop_region()
        .ok_or_else(|| "No selection".to_string())?;
    screenshot
        .crop(x, y, w, h)
        .map_err(|e| format!("Crop error: {}", e))
}

/// Copy current selection to clipboard
fn copy_selection_to_clipboard(canvas: &Canvas, screenshot: &Screenshot) -> ScreenshotResult<()> {
    let png_data = get_cropped_png(canvas, screenshot)?;
    clipboard::copy_image_to_clipboard(&png_data).map_err(|e| format!("Clipboard error: {}", e))
}

/// Save current selection to file
fn save_selection_to_file(canvas: &Canvas, screenshot: &Screenshot) -> ScreenshotResult<PathBuf> {
    let png_data = get_cropped_png(canvas, screenshot)?;
    let path =
        generate_screenshot_path().ok_or_else(|| "Could not determine save path".to_string())?;
    std::fs::write(&path, &png_data).map_err(|e| format!("Save error: {}", e))?;
    Ok(path)
}

fn main() -> glib::ExitCode {
    // Create the application
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_activate(build_ui);

    app.run()
}

/// Show a fatal error dialog and quit the application
fn show_fatal_error(app: &gtk4::Application, message: &str) {
    let dialog = gtk4::AlertDialog::builder()
        .message("Fatal Error")
        .detail(message)
        .modal(true)
        .build();

    // Clone app for the closure
    let app_clone = app.clone();
    dialog.show(None::<&gtk4::Window>);

    // Quit after a short delay to ensure dialog is shown
    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
        app_clone.quit();
    });
}

/// Create CSS styling for the button container
fn create_button_css() -> gtk4::CssProvider {
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(
        r#"
        .button-container {
            background-color: rgba(30, 30, 30, 0.9);
            border-radius: 9999px;
            border: 1px solid rgba(255, 255, 255, 0.1);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
        }
        .button-container button.circular {
            min-width: 40px;
            min-height: 40px;
            padding: 3px;
            border-radius: 50%;
            border: none;
            background-color: rgba(255, 255, 255, 0.1);
            color: #ffffff;
            box-shadow: none;
            transition: background-color 200ms ease;
        }
        .button-container button.circular:hover {
            background-color: rgba(255, 255, 255, 0.15);
        }
        .button-container button.circular:active {
            background-color: rgba(255, 255, 255, 0.2);
        }
        .button-container button.circular.suggested-action {
            background-color: #3584e4;
            color: #ffffff;
        }
        .button-container button.circular.suggested-action:hover {
            background-color: #4a9cf4;
        }
        .button-container button.circular.suggested-action:active {
            background-color: #2974d4;
        }
        .button-container button.circular.destructive-action {
            background-color: #e33b3b;
            color: #ffffff;
        }
        .button-container button.circular.destructive-action:hover {
            background-color: #f44b4b;
        }
        .button-container button.circular.destructive-action:active {
            background-color: #d32b2b;
        }
        "#,
    );
    css_provider
}

/// Create the button container with copy, save, and cancel buttons
fn create_button_container() -> (gtk4::Box, gtk4::Button, gtk4::Button, gtk4::Button) {
    let button_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    button_container.set_visible(false);
    button_container.add_css_class("button-container");

    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    button_box.set_halign(gtk4::Align::Center);
    button_box.set_valign(gtk4::Align::Center);
    button_box.set_margin_top(8);
    button_box.set_margin_bottom(8);
    button_box.set_margin_start(10);
    button_box.set_margin_end(10);

    // Create circular icon buttons using system symbolic icons
    let copy_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
    copy_btn.add_css_class("circular");
    copy_btn.add_css_class("suggested-action");
    copy_btn.set_tooltip_text(Some("Copy to clipboard"));

    let save_btn = gtk4::Button::from_icon_name("document-save-symbolic");
    save_btn.add_css_class("circular");
    save_btn.set_tooltip_text(Some("Save to file"));

    let cancel_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    cancel_btn.add_css_class("circular");
    cancel_btn.add_css_class("destructive-action");
    cancel_btn.set_tooltip_text(Some("Cancel"));

    button_box.append(&copy_btn);
    button_box.append(&save_btn);
    button_box.append(&cancel_btn);
    button_container.append(&button_box);

    (button_container, copy_btn, save_btn, cancel_btn)
}

/// Setup the selection change callback to update button position
fn setup_selection_callback(
    canvas: &Canvas,
    button_container: &gtk4::Box,
    fixed: &gtk4::Fixed,
    screen_width: i32,
    screen_height: i32,
) {
    let button_container_weak = button_container.downgrade();
    let fixed_weak = fixed.downgrade();

    canvas.set_on_selection_change(move |region| {
        let Some(button_container) = button_container_weak.upgrade() else {
            return;
        };
        let Some(fixed) = fixed_weak.upgrade() else {
            return;
        };

        if let Some((x, y, w, h)) = region {
            // Only show if selection is valid size
            if w >= 20 && h >= 20 {
                button_container.set_visible(true);

                // Calculate button container position
                let (_, natural) = button_container.preferred_size();
                let btn_width = natural.width() as f64;
                let btn_height = natural.height() as f64;

                // Center horizontally under the selection
                let center_x = x as f64 + (w as f64 / 2.0);
                let mut btn_x = center_x - (btn_width / 2.0);

                // Position below selection with some margin
                let margin = 12.0;
                let mut btn_y = (y + h) as f64 + margin;

                // If button would go off bottom, position above selection
                if btn_y + btn_height > screen_height as f64 - 10.0 {
                    btn_y = y as f64 - btn_height - margin;
                    // If still off screen (selection too high), put inside at bottom
                    if btn_y < 10.0 {
                        btn_y = (y + h) as f64 - btn_height - margin;
                    }
                }

                // Keep button container within horizontal screen bounds
                if btn_x < 10.0 {
                    btn_x = 10.0;
                }
                if btn_x + btn_width > screen_width as f64 - 10.0 {
                    btn_x = screen_width as f64 - btn_width - 10.0;
                }

                fixed.move_(&button_container, btn_x, btn_y);
            } else {
                button_container.set_visible(false);
            }
        } else {
            button_container.set_visible(false);
        }
    });
}

/// Setup keyboard shortcuts handler
fn setup_keyboard_shortcuts(
    window: &gtk4::ApplicationWindow,
    canvas: &Canvas,
    screenshot_data: &Rc<RefCell<Screenshot>>,
) {
    let key_controller = gtk4::EventControllerKey::new();
    let window_weak = window.downgrade();
    let canvas_weak = canvas.downgrade();
    let screenshot_ref = screenshot_data.clone();

    key_controller.connect_key_pressed(move |_, key, _, modifier| {
        let ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

        // ESC to cancel
        if key == gdk::Key::Escape {
            if let Some(w) = window_weak.upgrade() {
                w.close();
            }
            return glib::Propagation::Stop;
        }

        // Ctrl+A to select all
        if ctrl && (key == gdk::Key::a || key == gdk::Key::A) {
            if let Some(canvas) = canvas_weak.upgrade() {
                canvas.select_all();
            }
            return glib::Propagation::Stop;
        }

        // Ctrl+C to copy
        if ctrl && (key == gdk::Key::c || key == gdk::Key::C) {
            if let Some(canvas) = canvas_weak.upgrade() {
                let screenshot = screenshot_ref.borrow();
                if let Err(e) = copy_selection_to_clipboard(&canvas, &screenshot) {
                    eprintln!("{}", e);
                }
                drop(screenshot);
                if let Some(win) = window_weak.upgrade() {
                    win.close();
                }
            }
            return glib::Propagation::Stop;
        }

        // Ctrl+S to save
        if ctrl && (key == gdk::Key::s || key == gdk::Key::S) {
            if let Some(canvas) = canvas_weak.upgrade() {
                let screenshot = screenshot_ref.borrow();
                match save_selection_to_file(&canvas, &screenshot) {
                    Ok(path) => eprintln!("Saved to: {}", path.display()),
                    Err(e) => eprintln!("{}", e),
                }
                drop(screenshot);
                if let Some(win) = window_weak.upgrade() {
                    win.close();
                }
            }
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });

    window.add_controller(key_controller);
}

/// Connect button click handlers
fn connect_button_handlers(
    window: &gtk4::ApplicationWindow,
    canvas: &Canvas,
    screenshot_data: &Rc<RefCell<Screenshot>>,
    copy_btn: &gtk4::Button,
    save_btn: &gtk4::Button,
    cancel_btn: &gtk4::Button,
) {
    // Cancel button
    let window_weak = window.downgrade();
    cancel_btn.connect_clicked(move |_| {
        if let Some(w) = window_weak.upgrade() {
            w.close();
        }
    });

    // Copy button
    let canvas_weak = canvas.downgrade();
    let screenshot_ref = screenshot_data.clone();
    let window_weak = window.downgrade();
    copy_btn.connect_clicked(move |_| {
        if let Some(canvas) = canvas_weak.upgrade() {
            let screenshot = screenshot_ref.borrow();
            if let Err(e) = copy_selection_to_clipboard(&canvas, &screenshot) {
                eprintln!("{}", e);
            }
        }
        if let Some(w) = window_weak.upgrade() {
            w.close();
        }
    });

    // Save button
    let canvas_weak = canvas.downgrade();
    let screenshot_ref = screenshot_data.clone();
    let window_weak = window.downgrade();
    save_btn.connect_clicked(move |_| {
        let Some(canvas) = canvas_weak.upgrade() else {
            return;
        };
        let Some(win) = window_weak.upgrade() else {
            return;
        };

        let screenshot = screenshot_ref.borrow();
        match save_selection_to_file(&canvas, &screenshot) {
            Ok(path) => eprintln!("Saved to: {}", path.display()),
            Err(e) => eprintln!("{}", e),
        }
        drop(screenshot);
        win.close();
    });
}

fn build_ui(app: &gtk4::Application) {
    // Force Adwaita icon theme via GTK settings
    let settings = gtk4::Settings::default().expect("Could not get default settings");
    settings.set_gtk_icon_theme_name(Some("Adwaita"));

    // First, capture the screenshot before showing any UI
    let screenshot = match screenshot::Screenshot::capture() {
        Ok(s) => s,
        Err(e) => {
            show_fatal_error(app, &format!("Screenshot failed: {}", e));
            return;
        }
    };

    let screen_width = screenshot.width;
    let screen_height = screenshot.height;

    // Create the main window
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Waysnip")
        .build();

    // Setup layer shell
    if let Err(e) = window::setup_layer_shell(&window) {
        show_fatal_error(app, &format!("Layer shell error: {}", e));
        return;
    }

    // Use a Fixed container for precise positioning
    let fixed = gtk4::Fixed::new();

    // Create canvas and set the screenshot
    let canvas = Canvas::new();
    canvas.set_pixbuf(&screenshot.pixbuf);
    canvas.setup_controllers();
    canvas.set_size_request(screen_width, screen_height);
    fixed.put(&canvas, 0.0, 0.0);

    // Create button container
    let (button_container, copy_btn, save_btn, cancel_btn) = create_button_container();

    // Apply CSS styling
    let css_provider = create_button_css();
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Add button container to fixed
    fixed.put(&button_container, 0.0, 0.0);
    window.set_child(Some(&fixed));

    // Store screenshot data for later use
    let screenshot_data = Rc::new(RefCell::new(screenshot));

    // Setup callbacks and handlers
    setup_selection_callback(
        &canvas,
        &button_container,
        &fixed,
        screen_width,
        screen_height,
    );
    connect_button_handlers(
        &window,
        &canvas,
        &screenshot_data,
        &copy_btn,
        &save_btn,
        &cancel_btn,
    );
    setup_keyboard_shortcuts(&window, &canvas, &screenshot_data);

    window.present();
}
