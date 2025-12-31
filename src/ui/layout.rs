//! Simple stack-based layout system (similar to SwiftUI/Flutter)

/// Represents a rectangular region
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, row: u16, col: u16) -> bool {
        row >= self.y && row < self.y + self.height &&
        col >= self.x && col < self.x + self.width
    }
}

/// Size constraint for layout items
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum Size {
    /// Fixed size in characters
    Fixed(u16),
    /// Flexible - takes up remaining space proportionally (weight)
    Flex(u16),
    /// Percentage of parent
    Percent(u16),
}

impl Default for Size {
    fn default() -> Self {
        Size::Flex(1)
    }
}

/// A layout node
#[derive(Clone, Debug)]
pub enum LayoutNode {
    /// Vertical stack
    VStack {
        children: Vec<LayoutItem>,
        spacing: u16,
        padding: u16,
    },
    /// Horizontal stack
    HStack {
        children: Vec<LayoutItem>,
        spacing: u16,
        padding: u16,
    },
    /// Empty spacer
    Spacer,
    /// Leaf node (actual content)
    Leaf { id: String },
}

/// A layout item with size constraints
#[derive(Clone, Debug)]
pub struct LayoutItem {
    pub node: LayoutNode,
    pub width: Size,
    pub height: Size,
    pub min_width: u16,
    pub min_height: u16,
}

impl LayoutItem {
    pub fn vstack(children: Vec<LayoutItem>) -> Self {
        Self {
            node: LayoutNode::VStack { children, spacing: 0, padding: 0 },
            width: Size::Flex(1),
            height: Size::Flex(1),
            min_width: 0,
            min_height: 0,
        }
    }

    pub fn hstack(children: Vec<LayoutItem>) -> Self {
        Self {
            node: LayoutNode::HStack { children, spacing: 0, padding: 0 },
            width: Size::Flex(1),
            height: Size::Flex(1),
            min_width: 0,
            min_height: 0,
        }
    }

    pub fn spacer() -> Self {
        Self {
            node: LayoutNode::Spacer,
            width: Size::Flex(1),
            height: Size::Flex(1),
            min_width: 0,
            min_height: 0,
        }
    }

    pub fn leaf(id: impl Into<String>) -> Self {
        Self {
            node: LayoutNode::Leaf { id: id.into() },
            width: Size::Flex(1),
            height: Size::Fixed(1),
            min_width: 0,
            min_height: 0,
        }
    }

    // Builder methods
    pub fn width(mut self, w: Size) -> Self {
        self.width = w;
        self
    }

    pub fn height(mut self, h: Size) -> Self {
        self.height = h;
        self
    }

    pub fn fixed_width(mut self, w: u16) -> Self {
        self.width = Size::Fixed(w);
        self
    }

    pub fn fixed_height(mut self, h: u16) -> Self {
        self.height = Size::Fixed(h);
        self
    }

    pub fn min_size(mut self, w: u16, h: u16) -> Self {
        self.min_width = w;
        self.min_height = h;
        self
    }

    pub fn spacing(mut self, s: u16) -> Self {
        match &mut self.node {
            LayoutNode::VStack { spacing, .. } | LayoutNode::HStack { spacing, .. } => *spacing = s,
            _ => {}
        }
        self
    }

    pub fn padding(mut self, p: u16) -> Self {
        match &mut self.node {
            LayoutNode::VStack { padding, .. } | LayoutNode::HStack { padding, .. } => *padding = p,
            _ => {}
        }
        self
    }
}

/// Computed layout results
#[derive(Clone, Debug, Default)]
pub struct ComputedLayout {
    pub rects: std::collections::HashMap<String, Rect>,
}

impl ComputedLayout {
    pub fn get(&self, id: &str) -> Option<Rect> {
        self.rects.get(id).copied()
    }

    pub fn hit_test(&self, row: u16, col: u16) -> Option<String> {
        // Return the first matching element (could be improved with z-order)
        for (id, rect) in &self.rects {
            if rect.contains(row, col) {
                return Some(id.clone());
            }
        }
        None
    }
}

/// Compute layout within a given bounds
pub fn compute_layout(item: &LayoutItem, bounds: Rect) -> ComputedLayout {
    let mut result = ComputedLayout::default();
    compute_node(item, bounds, &mut result);
    result
}

fn compute_node(item: &LayoutItem, bounds: Rect, result: &mut ComputedLayout) {
    match &item.node {
        LayoutNode::VStack { children, spacing, padding } => {
            let inner = Rect {
                x: bounds.x + padding,
                y: bounds.y + padding,
                width: bounds.width.saturating_sub(padding * 2),
                height: bounds.height.saturating_sub(padding * 2),
            };

            let rects = distribute_vertical(children, inner, *spacing);
            for (child, rect) in children.iter().zip(rects.iter()) {
                compute_node(child, *rect, result);
            }
        }

        LayoutNode::HStack { children, spacing, padding } => {
            let inner = Rect {
                x: bounds.x + padding,
                y: bounds.y + padding,
                width: bounds.width.saturating_sub(padding * 2),
                height: bounds.height.saturating_sub(padding * 2),
            };

            let rects = distribute_horizontal(children, inner, *spacing);
            for (child, rect) in children.iter().zip(rects.iter()) {
                compute_node(child, *rect, result);
            }
        }

        LayoutNode::Spacer => {
            // Spacer doesn't produce output
        }

        LayoutNode::Leaf { id } => {
            result.rects.insert(id.clone(), bounds);
        }
    }
}

fn distribute_vertical(children: &[LayoutItem], bounds: Rect, spacing: u16) -> Vec<Rect> {
    if children.is_empty() {
        return vec![];
    }

    let total_spacing = spacing * (children.len() as u16).saturating_sub(1);
    let available_height = bounds.height.saturating_sub(total_spacing);

    // First pass: calculate fixed sizes and total flex weight
    let mut fixed_total = 0u16;
    let mut flex_total = 0u16;

    for child in children {
        match child.height {
            Size::Fixed(h) => fixed_total += h.max(child.min_height),
            Size::Flex(w) => flex_total += w,
            Size::Percent(p) => fixed_total += (bounds.height * p / 100).max(child.min_height),
        }
    }

    let flex_space = available_height.saturating_sub(fixed_total);

    // Second pass: assign heights
    let mut rects = Vec::with_capacity(children.len());
    let mut current_y = bounds.y;

    for child in children {
        let height = match child.height {
            Size::Fixed(h) => h.max(child.min_height),
            Size::Flex(w) => {
                if flex_total > 0 {
                    (flex_space * w / flex_total).max(child.min_height)
                } else {
                    child.min_height
                }
            }
            Size::Percent(p) => (bounds.height * p / 100).max(child.min_height),
        };

        // Calculate width for this child
        let width = match child.width {
            Size::Fixed(w) => w.max(child.min_width),
            Size::Flex(_) | Size::Percent(100) => bounds.width,
            Size::Percent(p) => (bounds.width * p / 100).max(child.min_width),
        };

        rects.push(Rect {
            x: bounds.x,
            y: current_y,
            width,
            height,
        });

        current_y += height + spacing;
    }

    rects
}

fn distribute_horizontal(children: &[LayoutItem], bounds: Rect, spacing: u16) -> Vec<Rect> {
    if children.is_empty() {
        return vec![];
    }

    let total_spacing = spacing * (children.len() as u16).saturating_sub(1);
    let available_width = bounds.width.saturating_sub(total_spacing);

    // First pass: calculate fixed sizes and total flex weight
    let mut fixed_total = 0u16;
    let mut flex_total = 0u16;

    for child in children {
        match child.width {
            Size::Fixed(w) => fixed_total += w.max(child.min_width),
            Size::Flex(w) => flex_total += w,
            Size::Percent(p) => fixed_total += (bounds.width * p / 100).max(child.min_width),
        }
    }

    let flex_space = available_width.saturating_sub(fixed_total);

    // Second pass: assign widths
    let mut rects = Vec::with_capacity(children.len());
    let mut current_x = bounds.x;

    for child in children {
        let width = match child.width {
            Size::Fixed(w) => w.max(child.min_width),
            Size::Flex(w) => {
                if flex_total > 0 {
                    (flex_space * w / flex_total).max(child.min_width)
                } else {
                    child.min_width
                }
            }
            Size::Percent(p) => (bounds.width * p / 100).max(child.min_width),
        };

        // Calculate height for this child
        let height = match child.height {
            Size::Fixed(h) => h.max(child.min_height),
            Size::Flex(_) | Size::Percent(100) => bounds.height,
            Size::Percent(p) => (bounds.height * p / 100).max(child.min_height),
        };

        rects.push(Rect {
            x: current_x,
            y: bounds.y,
            width,
            height,
        });

        current_x += width + spacing;
    }

    rects
}

/// Create a standard file dialog layout (full dialog including title bar)
pub fn file_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar row (on the border)
        LayoutItem::hstack(vec![
            LayoutItem::leaf("title_bar").width(Size::Flex(1)),
            LayoutItem::leaf("maximize").fixed_width(3),
            LayoutItem::leaf("close").fixed_width(3),
        ]).fixed_height(1),

        // Content area inside border
        LayoutItem::vstack(vec![
            // Filename row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("filename_label").fixed_width(12),
                LayoutItem::leaf("filename_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Directory row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("directory_label").fixed_width(12),
                LayoutItem::leaf("directory_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Labels row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("files_label").width(Size::Flex(1)),
                LayoutItem::leaf("dirs_label").width(Size::Flex(1)),
            ]).fixed_height(1).spacing(2),

            // Lists row (flexible height)
            LayoutItem::hstack(vec![
                LayoutItem::leaf("files_list").width(Size::Flex(1)).height(Size::Flex(1)).min_size(10, 3),
                LayoutItem::leaf("dirs_list").width(Size::Flex(1)).height(Size::Flex(1)).min_size(10, 3),
            ]).height(Size::Flex(1)).spacing(2),

            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::leaf("help_button").fixed_width(8),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),

        // Bottom border row (for resize handle)
        LayoutItem::hstack(vec![
            LayoutItem::spacer(),
            LayoutItem::leaf("resize_handle").fixed_width(2),
        ]).fixed_height(1),
    ])
}

/// Create a simple message dialog layout
#[allow(dead_code)]
pub fn message_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::hstack(vec![
            LayoutItem::leaf("title").width(Size::Flex(1)),
            LayoutItem::leaf("close").fixed_width(3),
        ]).fixed_height(1),

        LayoutItem::spacer().fixed_height(1),

        // Message content
        LayoutItem::leaf("content").height(Size::Flex(1)),

        LayoutItem::spacer().fixed_height(1),

        // Buttons row
        LayoutItem::hstack(vec![
            LayoutItem::spacer(),
            LayoutItem::leaf("ok_button").fixed_width(8),
            LayoutItem::spacer(),
        ]).fixed_height(1),
    ]).padding(1)
}

// ============================================================================
// Main Screen Layouts
// ============================================================================

/// Create the main screen layout
/// - menu_bar: Fixed height 1 at top
/// - output: Conditional, fixed height if shown (program output window)
/// - editor: Flex height (takes remaining space)
/// - immediate: Conditional, fixed height if shown
/// - status_bar: Fixed height 1 at bottom
pub fn main_screen_layout(
    show_immediate: bool,
    immediate_height: u16,
    immediate_maximized: bool,
    show_output: bool,
    output_height: u16,
) -> LayoutItem {
    let mut children = vec![
        LayoutItem::leaf("menu_bar").fixed_height(1),
    ];

    // Output window appears above editor when shown
    if show_output {
        children.push(LayoutItem::leaf("output").fixed_height(output_height));
    }

    // If immediate is maximized, it takes all the editor space
    if immediate_maximized && show_immediate {
        // Minimal editor (just 1 line visible)
        children.push(LayoutItem::leaf("editor").fixed_height(1));
        // Immediate takes remaining space
        children.push(LayoutItem::leaf("immediate").height(Size::Flex(1)));
    } else {
        // Normal layout
        children.push(LayoutItem::leaf("editor").height(Size::Flex(1)));

        if show_immediate {
            children.push(LayoutItem::leaf("immediate").fixed_height(immediate_height));
        }
    }

    children.push(LayoutItem::leaf("status_bar").fixed_height(1));

    LayoutItem::vstack(children)
}

/// Create the menu bar layout with dynamic menu titles
#[allow(dead_code)]
pub fn menu_bar_layout(menu_titles: &[&str]) -> LayoutItem {
    let mut children: Vec<LayoutItem> = Vec::new();

    // Add one character spacing on the left
    children.push(LayoutItem::spacer().fixed_width(1));

    for (i, title) in menu_titles.iter().enumerate() {
        // Each menu item is title length + 2 for padding
        let id = format!("menu_{}", i);
        children.push(LayoutItem::leaf(id).fixed_width(title.len() as u16 + 2));
    }

    // Add flex spacer to push everything left
    children.push(LayoutItem::spacer());

    LayoutItem::hstack(children)
}

/// Create editor layout
/// - title_bar: Fixed height 1 at top
/// - content_row: HStack of content (flex) + vscroll (fixed 1)
/// - hscroll_row: HStack of hscroll (flex) + corner (fixed 1)
#[allow(dead_code)]
pub fn editor_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area with vertical scrollbar
        LayoutItem::hstack(vec![
            LayoutItem::leaf("content").width(Size::Flex(1)),
            LayoutItem::leaf("vscroll").fixed_width(1),
        ]).height(Size::Flex(1)),

        // Horizontal scrollbar with corner
        LayoutItem::hstack(vec![
            LayoutItem::leaf("hscroll").width(Size::Flex(1)),
            LayoutItem::leaf("corner").fixed_width(1),
        ]).fixed_height(1),
    ])
}

/// Create vertical scrollbar layout (for internal scrollbar structure)
/// - up_arrow: Fixed 1
/// - track: Flex
/// - down_arrow: Fixed 1
#[allow(dead_code)]
pub fn vertical_scrollbar_layout(height: u16) -> LayoutItem {
    LayoutItem::vstack(vec![
        LayoutItem::leaf("up_arrow").fixed_height(1),
        LayoutItem::leaf("track").height(Size::Flex(1)),
        LayoutItem::leaf("down_arrow").fixed_height(1),
    ]).fixed_height(height)
}

/// Create horizontal scrollbar layout (for internal scrollbar structure)
/// - left_arrow: Fixed 1
/// - track: Flex
/// - right_arrow: Fixed 1
#[allow(dead_code)]
pub fn horizontal_scrollbar_layout(width: u16) -> LayoutItem {
    LayoutItem::hstack(vec![
        LayoutItem::leaf("left_arrow").fixed_width(1),
        LayoutItem::leaf("track").width(Size::Flex(1)),
        LayoutItem::leaf("right_arrow").fixed_width(1),
    ]).fixed_width(width)
}

/// Create status bar layout
/// - help_hint: Fixed width for F1=Help etc
/// - status_message: Flex (takes remaining)
/// - position: Fixed width for line:col
/// - mode: Fixed width for INS/OVR
#[allow(dead_code)]
pub fn status_bar_layout(width: u16) -> LayoutItem {
    LayoutItem::hstack(vec![
        LayoutItem::leaf("help_hint").fixed_width(49), // "<F1=Help> <F5=Run> <F10=Menu>"
        LayoutItem::leaf("status_message").width(Size::Flex(1)),
        LayoutItem::leaf("position").fixed_width(15), // " 00001:001  "
        LayoutItem::leaf("mode").fixed_width(4), // "INS" or "OVR"
    ]).fixed_width(width)
}

/// Create immediate window layout
/// - title_bar: Fixed 1
/// - output: Flex
/// - input_row: Fixed 1 (prompt + input field)
#[allow(dead_code)]
pub fn immediate_window_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        LayoutItem::leaf("title_bar").fixed_height(1),
        LayoutItem::leaf("output").height(Size::Flex(1)),
        LayoutItem::hstack(vec![
            LayoutItem::leaf("prompt").fixed_width(1), // ">"
            LayoutItem::leaf("input").width(Size::Flex(1)),
        ]).fixed_height(1),
    ])
}

// ============================================================================
// Dialog Layouts
// ============================================================================

/// Find dialog layout
pub fn find_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Find row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("find_label").fixed_width(8),
                LayoutItem::leaf("find_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Checkbox row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("case_checkbox").fixed_width(20),
                LayoutItem::leaf("whole_checkbox").fixed_width(18),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::leaf("help_button").fixed_width(8),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Replace dialog layout
pub fn replace_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Find row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("find_label").fixed_width(12),
                LayoutItem::leaf("find_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Replace row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("replace_label").fixed_width(12),
                LayoutItem::leaf("replace_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Checkbox row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("case_checkbox").fixed_width(20),
                LayoutItem::leaf("whole_checkbox").fixed_width(18),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("find_next_button").fixed_width(12),
                LayoutItem::leaf("replace_button").fixed_width(10),
                LayoutItem::leaf("replace_all_button").fixed_width(14),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(1),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Go To Line dialog layout
pub fn goto_line_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Line number row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("line_label").fixed_width(14),
                LayoutItem::leaf("line_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Welcome dialog layout
pub fn welcome_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::leaf("welcome_text").fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::leaf("copyright").fixed_height(1),
            LayoutItem::spacer().fixed_height(2),

            // Options on separate lines (vertically stacked)
            LayoutItem::leaf("start_button").fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::leaf("exit_button").fixed_height(1),

            LayoutItem::spacer().height(Size::Flex(1)),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Help dialog layout
pub fn help_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (scrollable)
        LayoutItem::leaf("content").height(Size::Flex(1)),

        // Status/navigation bar
        LayoutItem::leaf("nav_bar").fixed_height(1),
    ]).padding(1)
}

/// New Program confirmation dialog layout
#[allow(dead_code)]
pub fn new_program_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::leaf("message").height(Size::Flex(1)),
            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("yes_button").fixed_width(8),
                LayoutItem::leaf("no_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Print dialog layout
pub fn print_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Print option 1
            LayoutItem::leaf("option_selected").fixed_height(1),

            // Print option 2
            LayoutItem::leaf("option_range").fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Range input row (if applicable)
            LayoutItem::hstack(vec![
                LayoutItem::leaf("range_label").fixed_width(12),
                LayoutItem::leaf("range_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().height(Size::Flex(1)),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Confirm dialog layout (generic Yes/No/Cancel)
#[allow(dead_code)]
pub fn confirm_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::leaf("message").height(Size::Flex(1)),
            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("yes_button").fixed_width(8),
                LayoutItem::leaf("no_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Simple input dialog layout (NewSub, NewFunction, FindLabel, CommandArgs, HelpPath)
pub fn simple_input_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Input row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("input_label").fixed_width(12),
                LayoutItem::leaf("input_field").width(Size::Flex(1)),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

/// Display options dialog layout
pub fn display_options_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        // Title bar
        LayoutItem::leaf("title_bar").fixed_height(1),

        // Content area (padding 1)
        LayoutItem::vstack(vec![
            // Tab stops row
            LayoutItem::hstack(vec![
                LayoutItem::leaf("tabs_label").fixed_width(12),
                LayoutItem::leaf("tabs_field").fixed_width(6),
                LayoutItem::spacer(),
            ]).fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Scrollbars checkbox
            LayoutItem::leaf("scrollbars_checkbox").fixed_height(1),

            LayoutItem::spacer().fixed_height(1),

            // Color scheme label
            LayoutItem::leaf("scheme_label").fixed_height(1),

            // Color scheme options
            LayoutItem::leaf("scheme_blue").fixed_height(1),
            LayoutItem::leaf("scheme_dark").fixed_height(1),
            LayoutItem::leaf("scheme_light").fixed_height(1),

            LayoutItem::spacer().height(Size::Flex(1)),

            // Buttons row
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1).spacing(2),
        ]).height(Size::Flex(1)).padding(1),
    ])
}

// ============================================================================
// Menu Dropdown Layout
// ============================================================================

/// Create menu dropdown layout based on items
#[allow(dead_code)]
pub fn menu_dropdown_layout(items: &[&str]) -> LayoutItem {
    let children: Vec<LayoutItem> = items.iter().enumerate().map(|(i, item)| {
        if *item == "-" {
            LayoutItem::leaf(format!("separator_{}", i)).fixed_height(1)
        } else {
            LayoutItem::leaf(format!("item_{}", i)).fixed_height(1)
        }
    }).collect();

    LayoutItem::vstack(children)
}
