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
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const APP_ID: &str = "com.waysnip.Waysnip";

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

fn main() -> glib::ExitCode {
    // Create the application
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &gtk4::Application) {
    // First, capture the screenshot before showing any UI
    let screenshot = match screenshot::Screenshot::capture() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Screenshot failed: {}", e);
            std::process::exit(1);
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
        eprintln!("Layer shell error: {}", e);
        std::process::exit(1);
    }

    // Use a Fixed container for precise positioning
    let fixed = gtk4::Fixed::new();

    // Create canvas and set the screenshot
    let canvas = Canvas::new();
    canvas.set_pixbuf(&screenshot.pixbuf);
    canvas.setup_controllers();
    canvas.set_size_request(screen_width, screen_height);

    fixed.put(&canvas, 0.0, 0.0);

    // Create button box
    let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::Center);
    button_box.set_valign(gtk4::Align::Center);
    button_box.set_visible(false);

    // Create buttons
    let copy_btn = gtk4::Button::with_label("Copy");
    copy_btn.add_css_class("suggested-action");

    let save_btn = gtk4::Button::with_label("Save");

    let cancel_btn = gtk4::Button::with_label("Cancel");
    cancel_btn.add_css_class("destructive-action");

    button_box.append(&copy_btn);
    button_box.append(&save_btn);
    button_box.append(&cancel_btn);

    // Add button box to fixed (initial position doesn't matter, will be updated)
    fixed.put(&button_box, 0.0, 0.0);

    window.set_child(Some(&fixed));

    // Store screenshot data for later use
    let screenshot_data = Rc::new(RefCell::new(screenshot));

    // Setup selection change callback to update button position
    let button_box_weak = button_box.downgrade();
    let fixed_weak = fixed.downgrade();
    canvas.set_on_selection_change(move |region| {
        let Some(button_box) = button_box_weak.upgrade() else {
            return;
        };
        let Some(fixed) = fixed_weak.upgrade() else {
            return;
        };

        if let Some((x, y, w, h)) = region {
            // Only show if selection is valid size
            if w >= 20 && h >= 20 {
                button_box.set_visible(true);

                // Calculate button box position
                // Measure the button box size
                let (_, natural) = button_box.preferred_size();
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

                // Keep button box within horizontal screen bounds
                if btn_x < 10.0 {
                    btn_x = 10.0;
                }
                if btn_x + btn_width > screen_width as f64 - 10.0 {
                    btn_x = screen_width as f64 - btn_width - 10.0;
                }

                fixed.move_(&button_box, btn_x, btn_y);
            } else {
                button_box.set_visible(false);
            }
        } else {
            button_box.set_visible(false);
        }
    });

    // Connect button handlers
    let window_weak = window.downgrade();
    cancel_btn.connect_clicked(move |_| {
        if let Some(w) = window_weak.upgrade() {
            w.close();
        }
    });

    let canvas_weak = canvas.downgrade();
    let screenshot_ref = screenshot_data.clone();
    let window_weak = window.downgrade();
    copy_btn.connect_clicked(move |_| {
        if let Some(canvas) = canvas_weak.upgrade() {
            if let Some((x, y, w, h)) = canvas.get_crop_region() {
                let screenshot = screenshot_ref.borrow();
                match screenshot.crop(x, y, w, h) {
                    Ok(png_data) => {
                        if let Err(e) = clipboard::copy_image_to_clipboard(&png_data) {
                            eprintln!("Clipboard error: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Crop error: {}", e);
                    }
                }
            }
        }
        if let Some(w) = window_weak.upgrade() {
            w.close();
        }
    });

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
        let Some((x, y, w, h)) = canvas.get_crop_region() else {
            return;
        };

        let screenshot = screenshot_ref.borrow();
        let png_data = match screenshot.crop(x, y, w, h) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Crop error: {}", e);
                return;
            }
        };
        drop(screenshot);

        // Save to $HOME/Pictures with timestamp filename
        if let Some(path) = generate_screenshot_path() {
            if let Err(e) = std::fs::write(&path, &png_data) {
                eprintln!("Save error: {}", e);
            } else {
                eprintln!("Saved to: {}", path.display());
            }
        } else {
            eprintln!("Could not determine save path");
        }

        win.close();
    });

    // Keyboard shortcuts handler
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
                if let Some((x, y, w, h)) = canvas.get_crop_region() {
                    let screenshot = screenshot_ref.borrow();
                    if let Ok(png_data) = screenshot.crop(x, y, w, h) {
                        drop(screenshot);
                        if let Err(e) = clipboard::copy_image_to_clipboard(&png_data) {
                            eprintln!("Clipboard error: {}", e);
                        }
                        if let Some(win) = window_weak.upgrade() {
                            win.close();
                        }
                    }
                }
            }
            return glib::Propagation::Stop;
        }

        // Ctrl+S to save
        if ctrl && (key == gdk::Key::s || key == gdk::Key::S) {
            if let Some(canvas) = canvas_weak.upgrade() {
                if let Some((x, y, w, h)) = canvas.get_crop_region() {
                    let screenshot = screenshot_ref.borrow();
                    if let Ok(png_data) = screenshot.crop(x, y, w, h) {
                        drop(screenshot);
                        if let Some(path) = generate_screenshot_path() {
                            if let Err(e) = std::fs::write(&path, &png_data) {
                                eprintln!("Save error: {}", e);
                            } else {
                                eprintln!("Saved to: {}", path.display());
                            }
                        }
                        if let Some(win) = window_weak.upgrade() {
                            win.close();
                        }
                    }
                }
            }
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });
    window.add_controller(key_controller);

    window.present();
}
