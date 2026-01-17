//! Custom canvas widget for screenshot display and selection

use crate::selection::{DragMode, Rect, ResizeEdge, Selection};
use gdk_pixbuf::Pixbuf;
use gtk4::gdk;
use gtk4::graphene;
use gtk4::gsk;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, EventControllerMotion, GestureDrag};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// Callback type for selection change notifications
pub type SelectionChangeCallback = Box<dyn Fn(Option<(i32, i32, i32, i32)>)>;

mod imp {
    use super::*;

    pub struct Canvas {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub pixbuf: RefCell<Option<Pixbuf>>,
        pub selection: RefCell<Selection>,
        pub screen_width: Cell<f32>,
        pub screen_height: Cell<f32>,
        pub on_selection_change: RefCell<Option<SelectionChangeCallback>>,
        /// Current cursor position
        pub cursor_x: Cell<f32>,
        pub cursor_y: Cell<f32>,
        /// Whether cursor is currently over the widget
        pub cursor_inside: Cell<bool>,
        /// Cached cursor objects
        pub cursors: RefCell<HashMap<&'static str, gdk::Cursor>>,
        /// Current cursor name (to avoid unnecessary updates)
        pub current_cursor: RefCell<&'static str>,
    }

    impl Default for Canvas {
        fn default() -> Self {
            Self {
                texture: RefCell::new(None),
                pixbuf: RefCell::new(None),
                selection: RefCell::new(Selection::default()),
                screen_width: Cell::new(0.0),
                screen_height: Cell::new(0.0),
                on_selection_change: RefCell::new(None),
                cursor_x: Cell::new(0.0),
                cursor_y: Cell::new(0.0),
                cursor_inside: Cell::new(false),
                cursors: RefCell::new(HashMap::new()),
                current_cursor: RefCell::new("default"),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Canvas {
        const NAME: &'static str = "WaysnapCanvas";
        type Type = super::Canvas;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for Canvas {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_focusable(true);
            obj.set_can_focus(true);
        }
    }

    impl WidgetImpl for Canvas {
        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;

            // Draw background screenshot
            if let Some(ref texture) = *self.texture.borrow() {
                let rect = graphene::Rect::new(0.0, 0.0, width, height);
                snapshot.append_texture(texture, &rect);
            }

            // Dim color (semi-transparent black)
            let dim_color = gdk::RGBA::new(0.0, 0.0, 0.0, 0.5);

            // Draw dimming overlay with selection cutout
            let selection = self.selection.borrow();

            // Get predefined regions info for drawing
            let hovered_region = selection.hovered_region;
            let predefined_regions = selection.predefined_regions.clone();

            if let Some(sel_rect) = selection.rect {
                let sel_rect = sel_rect.normalized();

                // Draw the dimming in 4 parts around the selection
                // Top strip
                if sel_rect.y > 0.0 {
                    let top_rect = graphene::Rect::new(0.0, 0.0, width, sel_rect.y);
                    snapshot.append_color(&dim_color, &top_rect);
                }

                // Bottom strip
                let bottom_y = sel_rect.y + sel_rect.height;
                if bottom_y < height {
                    let bottom_rect = graphene::Rect::new(0.0, bottom_y, width, height - bottom_y);
                    snapshot.append_color(&dim_color, &bottom_rect);
                }

                // Left strip (between top and bottom)
                if sel_rect.x > 0.0 {
                    let left_rect =
                        graphene::Rect::new(0.0, sel_rect.y, sel_rect.x, sel_rect.height);
                    snapshot.append_color(&dim_color, &left_rect);
                }

                // Right strip (between top and bottom)
                let right_x = sel_rect.x + sel_rect.width;
                if right_x < width {
                    let right_rect =
                        graphene::Rect::new(right_x, sel_rect.y, width - right_x, sel_rect.height);
                    snapshot.append_color(&dim_color, &right_rect);
                }

                // Draw selection border
                let border_color = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);
                let border_width = 2.0;

                // Top border
                snapshot.append_color(
                    &border_color,
                    &graphene::Rect::new(
                        sel_rect.x - border_width,
                        sel_rect.y - border_width,
                        sel_rect.width + border_width * 2.0,
                        border_width,
                    ),
                );
                // Bottom border
                snapshot.append_color(
                    &border_color,
                    &graphene::Rect::new(
                        sel_rect.x - border_width,
                        sel_rect.y + sel_rect.height,
                        sel_rect.width + border_width * 2.0,
                        border_width,
                    ),
                );
                // Left border
                snapshot.append_color(
                    &border_color,
                    &graphene::Rect::new(
                        sel_rect.x - border_width,
                        sel_rect.y,
                        border_width,
                        sel_rect.height,
                    ),
                );
                // Right border
                snapshot.append_color(
                    &border_color,
                    &graphene::Rect::new(
                        sel_rect.x + sel_rect.width,
                        sel_rect.y,
                        border_width,
                        sel_rect.height,
                    ),
                );

                // Draw 4 corner handles only
                if let Some(handles) = selection.get_corner_handles() {
                    let handle_fill = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);
                    let handle_border_color = gdk::RGBA::new(0.3, 0.3, 0.3, 1.0);

                    for (_, handle_rect) in handles {
                        let rect = graphene::Rect::new(
                            handle_rect.x,
                            handle_rect.y,
                            handle_rect.width,
                            handle_rect.height,
                        );

                        // Draw border behind the handle
                        let outer_rect = graphene::Rect::new(
                            handle_rect.x - 1.0,
                            handle_rect.y - 1.0,
                            handle_rect.width + 2.0,
                            handle_rect.height + 2.0,
                        );
                        let outer_rounded = gsk::RoundedRect::from_rect(outer_rect, 4.0);
                        snapshot.push_rounded_clip(&outer_rounded);
                        snapshot.append_color(&handle_border_color, &outer_rect);
                        snapshot.pop();

                        // Draw handle fill
                        let rounded_rect = gsk::RoundedRect::from_rect(rect, 3.0);
                        snapshot.push_rounded_clip(&rounded_rect);
                        snapshot.append_color(&handle_fill, &rect);
                        snapshot.pop();
                    }
                }

                // Draw crosshair and magnifier based on drag mode
                if self.cursor_inside.get() {
                    let cursor_x = self.cursor_x.get();
                    let cursor_y = self.cursor_y.get();
                    let drag_mode = selection.drag_mode;

                    match drag_mode {
                        DragMode::None => {
                            // Check what would happen if user clicked here
                            let hover_mode = selection.hit_test(cursor_x, cursor_y);

                            // Only show crosshair/magnifier in dimmed area when not dragging
                            // and not hovering over resize handles/edges
                            if !sel_rect.contains(cursor_x, cursor_y) {
                                match hover_mode {
                                    DragMode::Creating => {
                                        // Cursor is in dimmed area, show magnifier
                                        self.draw_crosshair_and_magnifier(
                                            snapshot, width, height, cursor_x, cursor_y, true,
                                        );
                                    }
                                    _ => {
                                        // Cursor is over resize handle/edge, don't show magnifier
                                    }
                                }
                            }
                        }
                        DragMode::Creating | DragMode::Resizing(_) => {
                            // Get the snap position based on what's being resized
                            let snap_pos =
                                self.get_snap_position(&sel_rect, drag_mode, cursor_x, cursor_y);
                            self.draw_crosshair_and_magnifier(
                                snapshot, width, height, snap_pos.0, snap_pos.1,
                                true, // show crosshair
                            );
                        }
                        DragMode::Moving => {
                            // Don't show magnifier when moving
                        }
                    }
                }
            } else {
                // No selection yet - dim the entire screen
                let full_rect = graphene::Rect::new(0.0, 0.0, width, height);
                snapshot.append_color(&dim_color, &full_rect);

                // Draw predefined regions as clickable areas
                self.draw_predefined_regions(snapshot, &predefined_regions, hovered_region);

                // Draw crosshair and magnifier when no selection exists
                if self.cursor_inside.get() {
                    let cursor_x = self.cursor_x.get();
                    let cursor_y = self.cursor_y.get();
                    self.draw_crosshair_and_magnifier(
                        snapshot, width, height, cursor_x, cursor_y, true,
                    );
                }
            }
        }

        fn measure(&self, orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            let size = match orientation {
                gtk4::Orientation::Horizontal => self.screen_width.get() as i32,
                gtk4::Orientation::Vertical => self.screen_height.get() as i32,
                _ => 0,
            };
            (size, size, -1, -1)
        }
    }

    impl Canvas {
        /// Get the snap position for the magnifier based on drag mode
        /// Returns the position that should be centered in the magnifier
        fn get_snap_position(
            &self,
            sel_rect: &crate::selection::Rect,
            drag_mode: DragMode,
            cursor_x: f32,
            cursor_y: f32,
        ) -> (f32, f32) {
            match drag_mode {
                DragMode::Creating => {
                    // When creating, snap to the corner being dragged (opposite of start)
                    // The cursor position is the active corner
                    (cursor_x, cursor_y)
                }
                DragMode::Resizing(edge) => {
                    // Snap to the edge/corner being resized
                    match edge {
                        ResizeEdge::TopLeft => (sel_rect.x, sel_rect.y),
                        ResizeEdge::TopRight => (sel_rect.x + sel_rect.width, sel_rect.y),
                        ResizeEdge::BottomRight => {
                            (sel_rect.x + sel_rect.width, sel_rect.y + sel_rect.height)
                        }
                        ResizeEdge::BottomLeft => (sel_rect.x, sel_rect.y + sel_rect.height),
                        ResizeEdge::Top => (cursor_x, sel_rect.y),
                        ResizeEdge::Bottom => (cursor_x, sel_rect.y + sel_rect.height),
                        ResizeEdge::Left => (sel_rect.x, cursor_y),
                        ResizeEdge::Right => (sel_rect.x + sel_rect.width, cursor_y),
                    }
                }
                _ => (cursor_x, cursor_y),
            }
        }

        /// Draw crosshair lines and magnifier window
        fn draw_crosshair_and_magnifier(
            &self,
            snapshot: &gtk4::Snapshot,
            width: f32,
            height: f32,
            cursor_x: f32,
            cursor_y: f32,
            show_screen_crosshair: bool,
        ) {
            // Crosshair settings - dimmer at 0.8 opacity
            let line_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.8);
            let line_shadow_color = gdk::RGBA::new(0.0, 0.0, 0.0, 0.4);
            let line_width = 1.0;

            if show_screen_crosshair {
                // Draw shadow lines first (offset by 1 pixel for shadow effect)
                // Vertical line shadow
                snapshot.append_color(
                    &line_shadow_color,
                    &graphene::Rect::new(cursor_x + 1.0, 0.0, line_width, height),
                );
                // Horizontal line shadow
                snapshot.append_color(
                    &line_shadow_color,
                    &graphene::Rect::new(0.0, cursor_y + 1.0, width, line_width),
                );

                // Draw main crosshair lines
                // Vertical line (full height)
                snapshot.append_color(
                    &line_color,
                    &graphene::Rect::new(cursor_x, 0.0, line_width, height),
                );
                // Horizontal line (full width)
                snapshot.append_color(
                    &line_color,
                    &graphene::Rect::new(0.0, cursor_y, width, line_width),
                );
            }

            // Magnifier settings
            let pixel_size = 8.0; // Size of each zoomed pixel
            let pixels_x: i32 = 27; // Number of pixels horizontally (odd for center) - 1.8x bigger
            let pixels_y: i32 = 19; // Number of pixels vertically (odd for center) - 1.8x bigger
            let magnifier_width = pixel_size * pixels_x as f32;
            let magnifier_height = pixel_size * pixels_y as f32;
            let magnifier_margin = 20.0;
            let corner_radius = 8.0;
            let border_width = 2.0;

            // Calculate magnifier position (bottom-right of cursor by default)
            let mut mag_x = cursor_x + magnifier_margin;
            let mut mag_y = cursor_y + magnifier_margin;

            // Check for overflow and flip position if needed
            let overflow_right = mag_x + magnifier_width > width;
            let overflow_bottom = mag_y + magnifier_height > height;

            if overflow_right {
                mag_x = cursor_x - magnifier_margin - magnifier_width;
            }
            if overflow_bottom {
                mag_y = cursor_y - magnifier_margin - magnifier_height;
            }

            // Ensure we stay within bounds
            mag_x = mag_x.max(0.0).min(width - magnifier_width);
            mag_y = mag_y.max(0.0).min(height - magnifier_height);

            // Draw magnifier background/border
            let outer_rect = graphene::Rect::new(
                mag_x - border_width,
                mag_y - border_width,
                magnifier_width + border_width * 2.0,
                magnifier_height + border_width * 2.0,
            );
            let outer_rounded =
                gsk::RoundedRect::from_rect(outer_rect, corner_radius + border_width);
            let border_color = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);
            snapshot.push_rounded_clip(&outer_rounded);
            snapshot.append_color(&border_color, &outer_rect);
            snapshot.pop();

            // Draw magnified content with pixel grid
            let inner_rect = graphene::Rect::new(mag_x, mag_y, magnifier_width, magnifier_height);
            let inner_rounded = gsk::RoundedRect::from_rect(inner_rect, corner_radius);
            snapshot.push_rounded_clip(&inner_rounded);

            // Draw black background first
            let bg_color = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0);
            snapshot.append_color(&bg_color, &inner_rect);

            // Draw pixels from pixbuf using nearest-neighbor scaling
            if let Some(ref pixbuf) = *self.pixbuf.borrow() {
                let pb_width = pixbuf.width();
                let pb_height = pixbuf.height();

                // Center pixel position in source image
                let center_px = cursor_x.floor() as i32;
                let center_py = cursor_y.floor() as i32;

                // Calculate source region bounds
                let src_x = center_px - (pixels_x / 2);
                let src_y = center_py - (pixels_y / 2);

                // Clamp to valid pixbuf bounds
                let valid_src_x = src_x.max(0);
                let valid_src_y = src_y.max(0);
                let valid_end_x = (src_x + pixels_x).min(pb_width);
                let valid_end_y = (src_y + pixels_y).min(pb_height);
                let valid_width = (valid_end_x - valid_src_x).max(0);
                let valid_height = (valid_end_y - valid_src_y).max(0);

                if valid_width > 0 && valid_height > 0 {
                    // Extract sub-region and scale with nearest-neighbor
                    let sub_pixbuf =
                        pixbuf.new_subpixbuf(valid_src_x, valid_src_y, valid_width, valid_height);

                    let scale = pixel_size as i32;
                    if let Some(scaled) = sub_pixbuf.scale_simple(
                        valid_width * scale,
                        valid_height * scale,
                        gdk_pixbuf::InterpType::Nearest,
                    ) {
                        // Calculate offset for partial regions (when cursor is near edges)
                        let offset_x = (valid_src_x - src_x) as f32 * pixel_size;
                        let offset_y = (valid_src_y - src_y) as f32 * pixel_size;

                        let texture = gdk::Texture::for_pixbuf(&scaled);
                        let texture_rect = graphene::Rect::new(
                            mag_x + offset_x,
                            mag_y + offset_y,
                            (valid_width * scale) as f32,
                            (valid_height * scale) as f32,
                        );
                        snapshot.append_texture(&texture, &texture_rect);
                    }
                }

                // Draw pixel grid lines
                let grid_color = gdk::RGBA::new(0.3, 0.3, 0.3, 0.5);
                let grid_line_width = 1.0;

                // Vertical grid lines
                for i in 1..pixels_x {
                    let x = mag_x + i as f32 * pixel_size;
                    let grid_rect =
                        graphene::Rect::new(x, mag_y, grid_line_width, magnifier_height);
                    snapshot.append_color(&grid_color, &grid_rect);
                }

                // Horizontal grid lines
                for i in 1..pixels_y {
                    let y = mag_y + i as f32 * pixel_size;
                    let grid_rect = graphene::Rect::new(mag_x, y, magnifier_width, grid_line_width);
                    snapshot.append_color(&grid_color, &grid_rect);
                }
            }

            // Draw crosshair lines through center of magnifier (one grid block thick)
            let crosshair_color = gdk::RGBA::new(1.0, 0.2, 0.2, 0.5);
            let center_px_idx_x = pixels_x / 2;
            let center_px_idx_y = pixels_y / 2;

            // Vertical crosshair line (full height of magnifier, one pixel_size wide)
            snapshot.append_color(
                &crosshair_color,
                &graphene::Rect::new(
                    mag_x + center_px_idx_x as f32 * pixel_size,
                    mag_y,
                    pixel_size,
                    magnifier_height,
                ),
            );
            // Horizontal crosshair line (full width of magnifier, one pixel_size tall)
            snapshot.append_color(
                &crosshair_color,
                &graphene::Rect::new(
                    mag_x,
                    mag_y + center_px_idx_y as f32 * pixel_size,
                    magnifier_width,
                    pixel_size,
                ),
            );

            snapshot.pop();

            // Draw border highlight around center pixel
            let center_px_x = mag_x + (pixels_x / 2) as f32 * pixel_size;
            let center_px_y = mag_y + (pixels_y / 2) as f32 * pixel_size;
            let indicator_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.9);
            let indicator_width = 1.5;

            // Draw border around center pixel
            // Top border
            snapshot.append_color(
                &indicator_color,
                &graphene::Rect::new(center_px_x, center_px_y, pixel_size, indicator_width),
            );
            // Bottom border
            snapshot.append_color(
                &indicator_color,
                &graphene::Rect::new(
                    center_px_x,
                    center_px_y + pixel_size - indicator_width,
                    pixel_size,
                    indicator_width,
                ),
            );
            // Left border
            snapshot.append_color(
                &indicator_color,
                &graphene::Rect::new(center_px_x, center_px_y, indicator_width, pixel_size),
            );
            // Right border
            snapshot.append_color(
                &indicator_color,
                &graphene::Rect::new(
                    center_px_x + pixel_size - indicator_width,
                    center_px_y,
                    indicator_width,
                    pixel_size,
                ),
            );
        }

        /// Draw predefined regions as clickable/highlightable areas
        fn draw_predefined_regions(
            &self,
            snapshot: &gtk4::Snapshot,
            regions: &[Rect],
            hovered_region: Option<usize>,
        ) {
            if regions.is_empty() {
                return;
            }

            // Colors for predefined regions
            let normal_border_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.5);
            let hover_border_color = gdk::RGBA::new(1.0, 1.0, 1.0, 1.0);
            let hover_fill_color = gdk::RGBA::new(1.0, 1.0, 1.0, 0.15);
            let border_width = 2.0;

            for (i, region) in regions.iter().enumerate() {
                let is_hovered = hovered_region == Some(i);
                let border_color = if is_hovered {
                    &hover_border_color
                } else {
                    &normal_border_color
                };

                // Draw hover fill if this region is hovered
                if is_hovered {
                    let fill_rect =
                        graphene::Rect::new(region.x, region.y, region.width, region.height);
                    snapshot.append_color(&hover_fill_color, &fill_rect);
                }

                // Draw border
                // Top border
                snapshot.append_color(
                    border_color,
                    &graphene::Rect::new(region.x, region.y, region.width, border_width),
                );
                // Bottom border
                snapshot.append_color(
                    border_color,
                    &graphene::Rect::new(
                        region.x,
                        region.y + region.height - border_width,
                        region.width,
                        border_width,
                    ),
                );
                // Left border
                snapshot.append_color(
                    border_color,
                    &graphene::Rect::new(region.x, region.y, border_width, region.height),
                );
                // Right border
                snapshot.append_color(
                    border_color,
                    &graphene::Rect::new(
                        region.x + region.width - border_width,
                        region.y,
                        border_width,
                        region.height,
                    ),
                );
            }
        }
    }
}

glib::wrapper! {
    pub struct Canvas(ObjectSubclass<imp::Canvas>)
        @extends gtk4::Widget;
}

impl Canvas {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Set the screenshot pixbuf to display
    pub fn set_pixbuf(&self, pixbuf: &Pixbuf) {
        let imp = self.imp();
        let width = pixbuf.width() as f32;
        let height = pixbuf.height() as f32;

        // Store the pixbuf for magnifier use
        *imp.pixbuf.borrow_mut() = Some(pixbuf.clone());

        // Create texture from a new copy of pixbuf to avoid memory overlap issues
        let pixbuf_copy = pixbuf.copy().expect("Failed to copy pixbuf");
        let texture = gdk::Texture::for_pixbuf(&pixbuf_copy);
        *imp.texture.borrow_mut() = Some(texture);

        // Update dimensions
        imp.screen_width.set(width);
        imp.screen_height.set(height);

        // Initialize selection with screen dimensions
        *imp.selection.borrow_mut() = Selection::new(width, height);

        self.queue_draw();
    }

    /// Set predefined regions for quick selection
    pub fn set_predefined_regions(&self, regions: Vec<Rect>) {
        let mut selection = self.imp().selection.borrow_mut();
        selection.predefined_regions = regions;
    }

    /// Set callback for selection changes
    pub fn set_on_selection_change<F: Fn(Option<(i32, i32, i32, i32)>) + 'static>(
        &self,
        callback: F,
    ) {
        *self.imp().on_selection_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Notify listeners of selection change
    fn notify_selection_change(&self) {
        let imp = self.imp();
        let region = imp.selection.borrow().get_crop_region();
        if let Some(ref callback) = *imp.on_selection_change.borrow() {
            callback(region);
        }
    }

    /// Initialize cached cursors
    fn init_cursors(&self) {
        let imp = self.imp();
        let mut cursors = imp.cursors.borrow_mut();

        let cursor_names = [
            "default",
            "crosshair",
            "pointer",
            "grab",
            "grabbing",
            "nw-resize",
            "ne-resize",
            "sw-resize",
            "se-resize",
            "n-resize",
            "s-resize",
            "e-resize",
            "w-resize",
        ];

        for name in cursor_names {
            if let Some(cursor) = gdk::Cursor::from_name(name, None) {
                cursors.insert(name, cursor);
            }
        }
    }

    /// Set cursor by name (uses cache, only updates if changed)
    fn set_cursor_by_name(&self, name: &'static str) {
        let imp = self.imp();

        // Only update if cursor changed
        if *imp.current_cursor.borrow() == name {
            return;
        }

        *imp.current_cursor.borrow_mut() = name;

        if let Some(cursor) = imp.cursors.borrow().get(name) {
            self.set_cursor(Some(cursor));
        }
    }

    /// Setup gesture and motion controllers
    pub fn setup_controllers(&self) {
        // Initialize cursor cache
        self.init_cursors();
        // Drag gesture for selection
        let drag = GestureDrag::new();
        drag.set_button(gdk::BUTTON_PRIMARY);

        let canvas_weak = self.downgrade();
        drag.connect_drag_begin(move |_, x, y| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let mut selection = canvas.imp().selection.borrow_mut();

                // If no selection exists and clicking on a predefined region, select it
                if selection.rect.is_none() {
                    if let Some(index) = selection.find_predefined_region_at(x as f32, y as f32) {
                        selection.select_predefined_region(index);
                        drop(selection);
                        canvas.queue_draw();
                        canvas.notify_selection_change();
                        return;
                    }
                }

                selection.start_drag(x as f32, y as f32);
                let cursor_name = selection.cursor_for_position(x as f32, y as f32);
                drop(selection);

                canvas.set_cursor_by_name(cursor_name);
                canvas.queue_draw();
                canvas.notify_selection_change();
            }
        });

        let canvas_weak = self.downgrade();
        drag.connect_drag_update(move |gesture, offset_x, offset_y| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let (start_x, start_y) = gesture.start_point().unwrap_or((0.0, 0.0));
                let x = start_x + offset_x;
                let y = start_y + offset_y;

                let mut selection = canvas.imp().selection.borrow_mut();
                selection.update_drag(x as f32, y as f32);
                drop(selection);
                canvas.queue_draw();
                canvas.notify_selection_change();
            }
        });

        let canvas_weak = self.downgrade();
        drag.connect_drag_end(move |gesture, _, _| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let mut selection = canvas.imp().selection.borrow_mut();
                selection.end_drag();

                // Get cursor position to update cursor after drag ends
                let (x, y) = gesture.start_point().unwrap_or((0.0, 0.0));
                let cursor_name = selection.cursor_for_position(x as f32, y as f32);
                drop(selection);

                canvas.set_cursor_by_name(cursor_name);
                canvas.queue_draw();
                canvas.notify_selection_change();
            }
        });

        self.add_controller(drag);

        // Motion controller for cursor updates
        let motion = EventControllerMotion::new();
        let canvas_weak = self.downgrade();
        motion.connect_motion(move |_, x, y| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let imp = canvas.imp();

                // Update cursor position
                imp.cursor_x.set(x as f32);
                imp.cursor_y.set(y as f32);

                // Update hovered predefined region
                {
                    let mut selection = imp.selection.borrow_mut();
                    selection.update_hovered_region(x as f32, y as f32);
                }

                let selection = imp.selection.borrow();

                // Use pointer cursor when hovering over a predefined region
                let cursor_name = if selection.hovered_region.is_some() && selection.rect.is_none()
                {
                    "pointer"
                } else {
                    selection.cursor_for_position(x as f32, y as f32)
                };
                drop(selection);

                canvas.set_cursor_by_name(cursor_name);

                // Always redraw when cursor moves to update crosshair/magnifier
                canvas.queue_draw();
            }
        });

        // Track when cursor enters/leaves the widget
        let canvas_weak = self.downgrade();
        motion.connect_enter(move |_, x, y| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let imp = canvas.imp();
                imp.cursor_inside.set(true);
                imp.cursor_x.set(x as f32);
                imp.cursor_y.set(y as f32);
                canvas.queue_draw();
            }
        });

        let canvas_weak = self.downgrade();
        motion.connect_leave(move |_| {
            if let Some(canvas) = canvas_weak.upgrade() {
                canvas.imp().cursor_inside.set(false);
                canvas.queue_draw();
            }
        });

        self.add_controller(motion);

        // Set cursor_inside to true initially since the window covers the whole screen
        // and cursor is always "inside" when the app launches
        self.imp().cursor_inside.set(true);

        // Query initial cursor position after the widget is realized
        let canvas_weak = self.downgrade();
        self.connect_realize(move |_| {
            if let Some(canvas) = canvas_weak.upgrade() {
                // Get the pointer position from the display's default seat
                let display = canvas.display();
                if let Some(seat) = display.default_seat() {
                    if let Some(pointer) = seat.pointer() {
                        let (_, x, y) = pointer.surface_at_position();
                        let imp = canvas.imp();
                        imp.cursor_x.set(x as f32);
                        imp.cursor_y.set(y as f32);
                        canvas.queue_draw();
                    }
                }
            }
        });
    }

    /// Check if there's a valid selection
    pub fn has_valid_selection(&self) -> bool {
        self.imp().selection.borrow().has_valid_selection()
    }

    /// Get crop region
    pub fn get_crop_region(&self) -> Option<(i32, i32, i32, i32)> {
        self.imp().selection.borrow().get_crop_region()
    }

    /// Select the entire screen
    pub fn select_all(&self) {
        let imp = self.imp();
        let width = imp.screen_width.get();
        let height = imp.screen_height.get();

        let mut selection = imp.selection.borrow_mut();
        selection.rect = Some(crate::selection::Rect::new(0.0, 0.0, width, height));
        drop(selection);

        self.queue_draw();
        self.notify_selection_change();
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}
