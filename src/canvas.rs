//! Custom canvas widget for screenshot display and selection

use crate::selection::Selection;
use gdk_pixbuf::Pixbuf;
use gtk4::gdk;
use gtk4::graphene;
use gtk4::gsk;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, EventControllerMotion, GestureDrag};
use std::cell::{Cell, RefCell};

mod imp {
    use super::*;

    pub struct Canvas {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub selection: RefCell<Selection>,
        pub screen_width: Cell<f32>,
        pub screen_height: Cell<f32>,
        pub on_selection_change: RefCell<Option<Box<dyn Fn(Option<(i32, i32, i32, i32)>)>>>,
    }

    impl Default for Canvas {
        fn default() -> Self {
            Self {
                texture: RefCell::new(None),
                selection: RefCell::new(Selection::default()),
                screen_width: Cell::new(0.0),
                screen_height: Cell::new(0.0),
                on_selection_change: RefCell::new(None),
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
            } else {
                // No selection yet - dim the entire screen
                let full_rect = graphene::Rect::new(0.0, 0.0, width, height);
                snapshot.append_color(&dim_color, &full_rect);
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

        // Create texture from pixbuf
        let texture = gdk::Texture::for_pixbuf(pixbuf);
        *imp.texture.borrow_mut() = Some(texture);

        // Update dimensions
        imp.screen_width.set(width);
        imp.screen_height.set(height);

        // Initialize selection with screen dimensions
        *imp.selection.borrow_mut() = Selection::new(width, height);

        self.queue_draw();
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

    /// Setup gesture and motion controllers
    pub fn setup_controllers(&self) {
        // Drag gesture for selection
        let drag = GestureDrag::new();
        drag.set_button(gdk::BUTTON_PRIMARY);

        let canvas_weak = self.downgrade();
        drag.connect_drag_begin(move |_, x, y| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let mut selection = canvas.imp().selection.borrow_mut();
                selection.start_drag(x as f32, y as f32);
                drop(selection);
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
        drag.connect_drag_end(move |_, _, _| {
            if let Some(canvas) = canvas_weak.upgrade() {
                let mut selection = canvas.imp().selection.borrow_mut();
                selection.end_drag();
                drop(selection);
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
                let selection = canvas.imp().selection.borrow();
                let cursor_name = selection.cursor_for_position(x as f32, y as f32);
                drop(selection);

                if let Some(cursor) = gdk::Cursor::from_name(cursor_name, None) {
                    canvas.set_cursor(Some(&cursor));
                }
            }
        });

        self.add_controller(motion);
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
