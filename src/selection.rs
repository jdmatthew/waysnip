//! Selection box logic for handling drag, resize, and move operations

/// Size of resize handles in pixels
pub const HANDLE_SIZE: f32 = 14.0;

/// Edge grab zone width in pixels
pub const EDGE_GRAB_WIDTH: f32 = 8.0;

/// Minimum selection size in pixels
pub const MIN_SIZE: f32 = 20.0;

/// Which handle or edge is being dragged
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Top,
    Right,
    Bottom,
    Left,
}

impl ResizeEdge {
    /// Get the cursor name for this edge/handle
    pub fn cursor_name(&self) -> &'static str {
        match self {
            ResizeEdge::TopLeft => "nw-resize",
            ResizeEdge::TopRight => "ne-resize",
            ResizeEdge::BottomRight => "se-resize",
            ResizeEdge::BottomLeft => "sw-resize",
            ResizeEdge::Top => "n-resize",
            ResizeEdge::Right => "e-resize",
            ResizeEdge::Bottom => "s-resize",
            ResizeEdge::Left => "w-resize",
        }
    }
}

/// Current drag mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragMode {
    /// No drag operation
    #[default]
    None,
    /// Creating a new selection
    Creating,
    /// Moving the existing selection
    Moving,
    /// Resizing via a specific edge or corner
    Resizing(ResizeEdge),
}

/// A rectangle representing the selection area
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Normalize the rectangle so width and height are positive
    pub fn normalized(&self) -> Self {
        let (x, width) = if self.width < 0.0 {
            (self.x + self.width, -self.width)
        } else {
            (self.x, self.width)
        };
        let (y, height) = if self.height < 0.0 {
            (self.y + self.height, -self.height)
        } else {
            (self.y, self.height)
        };
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, px: f32, py: f32) -> bool {
        let norm = self.normalized();
        px >= norm.x && px <= norm.x + norm.width && py >= norm.y && py <= norm.y + norm.height
    }

    /// Get the right edge x coordinate
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge y coordinate
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Constrain the rectangle within bounds and enforce minimum size
    pub fn constrain(&self, screen_width: f32, screen_height: f32) -> Self {
        let mut rect = self.normalized();

        // Enforce minimum size
        rect.width = rect.width.max(MIN_SIZE);
        rect.height = rect.height.max(MIN_SIZE);

        // Keep within screen bounds
        rect.x = rect.x.max(0.0);
        rect.y = rect.y.max(0.0);

        if rect.x + rect.width > screen_width {
            rect.x = screen_width - rect.width;
        }
        if rect.y + rect.height > screen_height {
            rect.y = screen_height - rect.height;
        }

        // Final bounds check
        rect.x = rect.x.max(0.0);
        rect.y = rect.y.max(0.0);
        rect.width = rect.width.min(screen_width);
        rect.height = rect.height.min(screen_height);

        rect
    }
}

/// Selection state management
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Current selection rectangle (normalized)
    pub rect: Option<Rect>,
    /// Screen dimensions for bounds checking
    pub screen_width: f32,
    pub screen_height: f32,
    /// Current drag mode
    pub drag_mode: DragMode,
    /// Starting point of drag
    pub drag_start: (f32, f32),
    /// Original rect when drag started
    pub drag_start_rect: Option<Rect>,
}

impl Selection {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            rect: None,
            screen_width,
            screen_height,
            drag_mode: DragMode::None,
            drag_start: (0.0, 0.0),
            drag_start_rect: None,
        }
    }

    /// Get the 4 corner handle rectangles for the current selection
    pub fn get_corner_handles(&self) -> Option<[(ResizeEdge, Rect); 4]> {
        let rect = self.rect?.normalized();
        let hs = HANDLE_SIZE;
        let hhs = hs / 2.0;

        Some([
            (
                ResizeEdge::TopLeft,
                Rect::new(rect.x - hhs, rect.y - hhs, hs, hs),
            ),
            (
                ResizeEdge::TopRight,
                Rect::new(rect.right() - hhs, rect.y - hhs, hs, hs),
            ),
            (
                ResizeEdge::BottomRight,
                Rect::new(rect.right() - hhs, rect.bottom() - hhs, hs, hs),
            ),
            (
                ResizeEdge::BottomLeft,
                Rect::new(rect.x - hhs, rect.bottom() - hhs, hs, hs),
            ),
        ])
    }

    /// Determine which corner handle (if any) is under the given point
    fn hit_test_corner(&self, x: f32, y: f32) -> Option<ResizeEdge> {
        let handles = self.get_corner_handles()?;
        for (edge, rect) in handles {
            if rect.contains(x, y) {
                return Some(edge);
            }
        }
        None
    }

    /// Determine which edge (if any) is under the given point
    fn hit_test_edge(&self, x: f32, y: f32) -> Option<ResizeEdge> {
        let rect = self.rect?.normalized();
        let grab = EDGE_GRAB_WIDTH;

        // Check if point is near any edge (but not in corners - those are handled separately)
        let in_horizontal =
            x >= rect.x + HANDLE_SIZE / 2.0 && x <= rect.right() - HANDLE_SIZE / 2.0;
        let in_vertical = y >= rect.y + HANDLE_SIZE / 2.0 && y <= rect.bottom() - HANDLE_SIZE / 2.0;

        // Top edge
        if in_horizontal && y >= rect.y - grab && y <= rect.y + grab {
            return Some(ResizeEdge::Top);
        }
        // Bottom edge
        if in_horizontal && y >= rect.bottom() - grab && y <= rect.bottom() + grab {
            return Some(ResizeEdge::Bottom);
        }
        // Left edge
        if in_vertical && x >= rect.x - grab && x <= rect.x + grab {
            return Some(ResizeEdge::Left);
        }
        // Right edge
        if in_vertical && x >= rect.right() - grab && x <= rect.right() + grab {
            return Some(ResizeEdge::Right);
        }

        None
    }

    /// Determine what drag mode should be used for a click at the given point
    pub fn hit_test(&self, x: f32, y: f32) -> DragMode {
        // First check corner handles (highest priority)
        if let Some(edge) = self.hit_test_corner(x, y) {
            return DragMode::Resizing(edge);
        }

        // Then check edges
        if let Some(edge) = self.hit_test_edge(x, y) {
            return DragMode::Resizing(edge);
        }

        // Then check if inside selection (for moving)
        if let Some(ref rect) = self.rect {
            if rect.normalized().contains(x, y) {
                return DragMode::Moving;
            }
        }

        // Otherwise, create new selection
        DragMode::Creating
    }

    /// Get cursor name for the given position
    pub fn cursor_for_position(&self, x: f32, y: f32) -> &'static str {
        // Check corners first
        if let Some(edge) = self.hit_test_corner(x, y) {
            return edge.cursor_name();
        }

        // Check edges
        if let Some(edge) = self.hit_test_edge(x, y) {
            return edge.cursor_name();
        }

        // Check if inside selection
        if let Some(ref rect) = self.rect {
            if rect.normalized().contains(x, y) {
                return "move";
            }
        }

        "crosshair"
    }

    /// Start a drag operation
    pub fn start_drag(&mut self, x: f32, y: f32) {
        self.drag_mode = self.hit_test(x, y);
        self.drag_start = (x, y);
        self.drag_start_rect = self.rect;

        if self.drag_mode == DragMode::Creating {
            self.rect = Some(Rect::new(x, y, 0.0, 0.0));
        }
    }

    /// Update drag operation
    pub fn update_drag(&mut self, x: f32, y: f32) {
        let (sx, sy) = self.drag_start;
        let dx = x - sx;
        let dy = y - sy;

        match self.drag_mode {
            DragMode::None => {}
            DragMode::Creating => {
                self.rect = Some(Rect::new(sx, sy, dx, dy));
            }
            DragMode::Moving => {
                if let Some(start_rect) = self.drag_start_rect {
                    let mut new_rect = Rect::new(
                        start_rect.x + dx,
                        start_rect.y + dy,
                        start_rect.width,
                        start_rect.height,
                    );
                    // Constrain to screen
                    new_rect = new_rect.constrain(self.screen_width, self.screen_height);
                    self.rect = Some(new_rect);
                }
            }
            DragMode::Resizing(edge) => {
                if let Some(start_rect) = self.drag_start_rect {
                    let rect = self.apply_resize(start_rect, edge, dx, dy);
                    self.rect = Some(rect);
                }
            }
        }
    }

    /// Apply resize operation based on edge
    fn apply_resize(&self, start: Rect, edge: ResizeEdge, dx: f32, dy: f32) -> Rect {
        let mut rect = start;

        match edge {
            ResizeEdge::TopLeft => {
                rect.x = start.x + dx;
                rect.y = start.y + dy;
                rect.width = start.width - dx;
                rect.height = start.height - dy;
            }
            ResizeEdge::Top => {
                rect.y = start.y + dy;
                rect.height = start.height - dy;
            }
            ResizeEdge::TopRight => {
                rect.y = start.y + dy;
                rect.width = start.width + dx;
                rect.height = start.height - dy;
            }
            ResizeEdge::Right => {
                rect.width = start.width + dx;
            }
            ResizeEdge::BottomRight => {
                rect.width = start.width + dx;
                rect.height = start.height + dy;
            }
            ResizeEdge::Bottom => {
                rect.height = start.height + dy;
            }
            ResizeEdge::BottomLeft => {
                rect.x = start.x + dx;
                rect.width = start.width - dx;
                rect.height = start.height + dy;
            }
            ResizeEdge::Left => {
                rect.x = start.x + dx;
                rect.width = start.width - dx;
            }
        }

        // Normalize and constrain
        rect.normalized()
            .constrain(self.screen_width, self.screen_height)
    }

    /// End drag operation
    pub fn end_drag(&mut self) {
        if let Some(ref mut rect) = self.rect {
            *rect = rect
                .normalized()
                .constrain(self.screen_width, self.screen_height);
        }
        self.drag_mode = DragMode::None;
        self.drag_start_rect = None;
    }

    /// Get the current selection as integer values for cropping
    pub fn get_crop_region(&self) -> Option<(i32, i32, i32, i32)> {
        let rect = self.rect?.normalized();
        Some((
            rect.x.round() as i32,
            rect.y.round() as i32,
            rect.width.round() as i32,
            rect.height.round() as i32,
        ))
    }

    /// Check if there's a valid selection
    pub fn has_valid_selection(&self) -> bool {
        if let Some(rect) = self.rect {
            let norm = rect.normalized();
            norm.width >= MIN_SIZE && norm.height >= MIN_SIZE
        } else {
            false
        }
    }
}
