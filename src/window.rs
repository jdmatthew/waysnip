//! Layer shell window setup for Wayland

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

/// Error type for window operations
#[derive(Debug)]
pub enum WindowError {
    LayerShellNotSupported,
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::LayerShellNotSupported => {
                write!(f, "Layer shell is not supported by the compositor")
            }
        }
    }
}

impl std::error::Error for WindowError {}

/// Check if layer shell is supported by the current compositor
pub fn is_layer_shell_supported() -> bool {
    gtk4_layer_shell::is_supported()
}

/// Configure a window as a layer shell overlay window
pub fn setup_layer_shell(window: &gtk4::ApplicationWindow) -> Result<(), WindowError> {
    if !is_layer_shell_supported() {
        return Err(WindowError::LayerShellNotSupported);
    }

    // Initialize layer shell for this window
    window.init_layer_shell();

    // Set namespace for the window (for identification by compositor)
    window.set_namespace("waysnip");

    // Set layer to overlay (topmost layer)
    window.set_layer(Layer::Overlay);

    // Set exclusive zone to -1 to not reserve any space
    window.set_exclusive_zone(-1);

    // Anchor to all edges for fullscreen coverage
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);

    // Request exclusive keyboard input
    window.set_keyboard_mode(KeyboardMode::Exclusive);

    // Window styling
    window.set_decorated(false);

    Ok(())
}
