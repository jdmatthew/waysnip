//! Screenshot capture functionality using libwayshot (wlr-screencopy protocol)

use gdk_pixbuf::{Colorspace, Pixbuf};
use libwayshot::WayshotConnection;

/// Error type for screenshot operations
#[derive(Debug)]
pub enum ScreenshotError {
    WayshotError(String),
    PixbufError(String),
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenshotError::WayshotError(msg) => {
                write!(f, "Screenshot capture failed: {}", msg)
            }
            ScreenshotError::PixbufError(msg) => write!(f, "Failed to create image: {}", msg),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// Captured screenshot data
pub struct Screenshot {
    /// Loaded pixbuf for display
    pub pixbuf: Pixbuf,
    /// Screen width
    pub width: i32,
    /// Screen height
    pub height: i32,
}

impl Screenshot {
    /// Capture a screenshot using wlr-screencopy protocol via libwayshot
    pub fn capture() -> Result<Self, ScreenshotError> {
        // Connect to Wayland and capture screenshot
        let wayshot =
            WayshotConnection::new().map_err(|e| ScreenshotError::WayshotError(e.to_string()))?;

        // Capture all outputs (no cursor overlay)
        let image = wayshot
            .screenshot_all(false)
            .map_err(|e| ScreenshotError::WayshotError(e.to_string()))?;

        // Convert DynamicImage to RGBA8
        let rgba_image = image.to_rgba8();
        let width = rgba_image.width() as i32;
        let height = rgba_image.height() as i32;
        let pixels = rgba_image.into_raw();

        // Create Pixbuf from raw RGBA data
        let pixbuf = Pixbuf::from_bytes(
            &glib::Bytes::from(&pixels),
            Colorspace::Rgb,
            true, // has_alpha
            8,    // bits_per_sample
            width,
            height,
            width * 4, // rowstride (4 bytes per pixel: RGBA)
        );

        Ok(Screenshot {
            pixbuf,
            width,
            height,
        })
    }

    /// Crop the screenshot to the given rectangle
    pub fn crop(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<Vec<u8>, ScreenshotError> {
        // Clamp to valid bounds
        let x = x.max(0).min(self.width - 1);
        let y = y.max(0).min(self.height - 1);
        let width = width.min(self.width - x).max(1);
        let height = height.min(self.height - y).max(1);

        // Create a new subpixbuf for the selection
        let cropped = self.pixbuf.new_subpixbuf(x, y, width, height);

        // Save to PNG bytes
        cropped
            .save_to_bufferv("png", &[])
            .map_err(|e: glib::Error| ScreenshotError::PixbufError(e.to_string()))
    }
}
