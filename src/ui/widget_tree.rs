//! Widget tree system for hierarchical UI composition
//!
//! The widget tree provides:
//! - Hierarchical widget composition (containers and leaves)
//! - Automatic layout computation via the layout engine
//! - Recursive drawing with theme support
//! - Event routing with focus management
//! - Tab navigation between focusable widgets

use crate::input::InputEvent;
use crate::screen::Screen;
use super::layout::{Rect, LayoutItem, Size, SizeHint, compute_child_bounds};
use super::theme::Theme;
use super::widget::{EventResult, mouse_position};
use std::any::Any;

/// Event dispatch phase for widget tree routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// A node in the widget tree - either a leaf widget or a container
pub enum WidgetNode {
    /// A leaf widget that draws itself
    Leaf {
        id: String,
        widget: Box<dyn TreeWidget>,
    },
    /// A container with layout and children
    Container {
        id: String,
        layout: ContainerLayout,
        children: Vec<WidgetNode>,
        /// Optional chrome drawing function (borders, backgrounds, etc.)
        chrome: Option<Box<dyn Fn(&mut Screen, Rect, &Theme)>>,
        /// Optional event handler for container-level behavior
        handler: Option<Box<dyn FnMut(&InputEvent, Rect, EventPhase) -> EventResult>>,
    },
}

/// Container layout direction
#[derive(Clone, Debug)]
pub enum ContainerLayout {
    /// Vertical stack (children arranged top to bottom)
    VStack { spacing: u16, padding: u16 },
    /// Horizontal stack (children arranged left to right)
    HStack { spacing: u16, padding: u16 },
}

impl WidgetNode {
    /// Create a new leaf widget node
    pub fn leaf(id: impl Into<String>, widget: impl TreeWidget + 'static) -> Self {
        WidgetNode::Leaf {
            id: id.into(),
            widget: Box::new(widget),
        }
    }

    /// Create a new vertical stack container
    pub fn vstack(id: impl Into<String>) -> ContainerBuilder {
        ContainerBuilder {
            id: id.into(),
            layout: ContainerLayout::VStack { spacing: 0, padding: 0 },
            children: Vec::new(),
            chrome: None,
            handler: None,
        }
    }

    /// Create a new horizontal stack container
    pub fn hstack(id: impl Into<String>) -> ContainerBuilder {
        ContainerBuilder {
            id: id.into(),
            layout: ContainerLayout::HStack { spacing: 0, padding: 0 },
            children: Vec::new(),
            chrome: None,
            handler: None,
        }
    }

    /// Get the ID of this node
    pub fn id(&self) -> &str {
        match self {
            WidgetNode::Leaf { id, .. } => id,
            WidgetNode::Container { id, .. } => id,
        }
    }


    /// Convert this node into a LayoutItem for the layout engine
    fn to_layout_item(&self) -> LayoutItem {
        match self {
            WidgetNode::Leaf { id, widget } => {
                let hint = widget.size_hint();
                let mut item = LayoutItem::leaf(id.clone());
                if hint.min_width > 0 {
                    item.min_width = hint.min_width;
                }
                if widget.wants_tight_width() && hint.min_width > 0 {
                    item.width = Size::Fixed(hint.min_width);
                }
                if hint.min_height > 0 {
                    item.min_height = hint.min_height;
                }
                if hint.flex > 0 {
                    item.height = Size::Flex(hint.flex);
                } else if hint.min_height > 0 {
                    item.height = Size::Fixed(hint.min_height);
                }
                item
            }
            WidgetNode::Container { children, layout, .. } => {
                let child_items: Vec<LayoutItem> = children
                    .iter()
                    .map(|c| c.to_layout_item())
                    .collect();

                match layout {
                    ContainerLayout::VStack { spacing, padding } => {
                        LayoutItem::vstack(child_items)
                            .spacing(*spacing)
                            .padding(*padding)
                    }
                    ContainerLayout::HStack { spacing, padding } => {
                        LayoutItem::hstack(child_items)
                            .spacing(*spacing)
                            .padding(*padding)
                    }
                }
            }
        }
    }

    /// Draw this node and all children
    pub fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        screen.push_clip(bounds.y, bounds.x, bounds.width, bounds.height);
        match self {
            WidgetNode::Leaf { widget, .. } => {
                widget.draw(screen, bounds, theme);
            }
            WidgetNode::Container { children, chrome, layout, .. } => {
                // Draw chrome (borders, background) if present
                if let Some(chrome_fn) = chrome {
                    chrome_fn(screen, bounds, theme);
                }

                // Get the padding from the layout
                let padding = match layout {
                    ContainerLayout::VStack { padding, .. } => *padding,
                    ContainerLayout::HStack { padding, .. } => *padding,
                };

                // Compute bounds for each child and draw recursively
                let layout_item = self.to_layout_item();
                let child_bounds = compute_child_bounds(&layout_item, bounds);
                for (child, mut rect) in children.iter().zip(child_bounds.into_iter()) {
                    // Check if this child widget wants full-bleed (extend to container edges)
                    let wants_full_bleed = match child {
                        WidgetNode::Leaf { widget, .. } => widget.wants_full_bleed(),
                        _ => false,
                    };
                    if wants_full_bleed && padding > 0 {
                        // Extend x to container edge and width to full container width
                        rect.x = bounds.x;
                        rect.width = bounds.width;
                    }
                    child.draw(screen, rect, theme);
                }
            }
        }
        screen.pop_clip();
    }

    fn find_path_at(&self, row: u16, col: u16, bounds: Rect) -> Option<Vec<String>> {
        match self {
            WidgetNode::Leaf { id, .. } => {
                if bounds.contains(row, col) {
                    Some(vec![id.clone()])
                } else {
                    None
                }
            }
            WidgetNode::Container { id, children, .. } => {
                let layout_item = self.to_layout_item();
                let child_bounds = compute_child_bounds(&layout_item, bounds);
                for (child, rect) in children.iter().zip(child_bounds.into_iter()) {
                    if !rect.contains(row, col) {
                        continue;
                    }
                    if let Some(mut path) = child.find_path_at(row, col, rect) {
                        path.insert(0, id.clone());
                        return Some(path);
                    }
                }
                None
            }
        }
    }

    fn dispatch_event(
        &mut self,
        event: &InputEvent,
        bounds: Rect,
        target_path: &[String],
        depth: usize,
    ) -> EventResult {
        if target_path.get(depth).map(|s| s.as_str()) != Some(self.id()) {
            return EventResult::Ignored;
        }

        let is_target = depth + 1 == target_path.len();

        match self {
            WidgetNode::Leaf { widget, .. } => {
                if !is_target {
                    return EventResult::Ignored;
                }
                widget.handle_event(event, bounds, EventPhase::Target)
            }
            WidgetNode::Container { children, layout, handler, .. } => {
                if let Some(handler_fn) = handler.as_mut() {
                    let result = handler_fn(event, bounds, EventPhase::Capture);
                    if result.is_consumed() {
                        return result;
                    }
                }

                if is_target {
                    if let Some(handler_fn) = handler.as_mut() {
                        let result = handler_fn(event, bounds, EventPhase::Target);
                        if result.is_consumed() {
                            return result;
                        }
                    }
                } else {
                    let child_items: Vec<LayoutItem> = children
                        .iter()
                        .map(|c| c.to_layout_item())
                        .collect();

                    let layout_item = match layout {
                        ContainerLayout::VStack { spacing, padding } => {
                            LayoutItem::vstack(child_items)
                                .spacing(*spacing)
                                .padding(*padding)
                        }
                        ContainerLayout::HStack { spacing, padding } => {
                            LayoutItem::hstack(child_items)
                                .spacing(*spacing)
                                .padding(*padding)
                        }
                    };

                    let child_bounds = compute_child_bounds(&layout_item, bounds);
                    for (child, rect) in children.iter_mut().zip(child_bounds.into_iter()) {
                        if child.id() == target_path[depth + 1] {
                            let result = child.dispatch_event(event, rect, target_path, depth + 1);
                            if result.is_consumed() {
                                return result;
                            }
                            break;
                        }
                    }
                }

                if let Some(handler_fn) = handler.as_mut() {
                    let result = handler_fn(event, bounds, EventPhase::Bubble);
                    if result.is_consumed() {
                        return result;
                    }
                }

                EventResult::Ignored
            }
        }
    }

    /// Find a widget by path (e.g., ["container", "child"])
    pub fn get_widget(&self, path: &[&str]) -> Option<&dyn TreeWidget> {
        if path.is_empty() {
            return None;
        }

        match self {
            WidgetNode::Leaf { id, widget } if id == path[0] => {
                if path.len() == 1 {
                    Some(widget.as_ref())
                } else {
                    None
                }
            }
            WidgetNode::Container { id, children, .. } if id == path[0] => {
                if path.len() == 1 {
                    None // Container itself is not a widget
                } else {
                    for child in children {
                        if let Some(w) = child.get_widget(&path[1..]) {
                            return Some(w);
                        }
                    }
                    None
                }
            }
            _ => None,
        }
    }

    /// Find a mutable widget by path
    pub fn get_widget_mut(&mut self, path: &[&str]) -> Option<&mut dyn TreeWidget> {
        if path.is_empty() {
            return None;
        }

        match self {
            WidgetNode::Leaf { id, widget } if id == path[0] => {
                if path.len() == 1 {
                    Some(widget.as_mut())
                } else {
                    None
                }
            }
            WidgetNode::Container { id, children, .. } if id == path[0] => {
                if path.len() == 1 {
                    None
                } else {
                    for child in children {
                        if let Some(w) = child.get_widget_mut(&path[1..]) {
                            return Some(w);
                        }
                    }
                    None
                }
            }
            _ => None,
        }
    }

    /// Collect all focusable widget paths in order
    pub fn collect_focusable(&self, prefix: &[String]) -> Vec<Vec<String>> {
        let mut result = Vec::new();
        let mut current_path = prefix.to_vec();
        current_path.push(self.id().to_string());

        match self {
            WidgetNode::Leaf { widget, .. } => {
                if widget.focusable() {
                    result.push(current_path);
                }
            }
            WidgetNode::Container { children, .. } => {
                for child in children {
                    result.extend(child.collect_focusable(&current_path));
                }
            }
        }

        result
    }
}

/// Builder for creating container nodes
pub struct ContainerBuilder {
    id: String,
    layout: ContainerLayout,
    children: Vec<WidgetNode>,
    chrome: Option<Box<dyn Fn(&mut Screen, Rect, &Theme)>>,
    handler: Option<Box<dyn FnMut(&InputEvent, Rect, EventPhase) -> EventResult>>,
}

impl ContainerBuilder {
    /// Set spacing between children
    pub fn spacing(mut self, spacing: u16) -> Self {
        match &mut self.layout {
            ContainerLayout::VStack { spacing: s, .. } => *s = spacing,
            ContainerLayout::HStack { spacing: s, .. } => *s = spacing,
        }
        self
    }

    /// Set padding around children
    pub fn padding(mut self, padding: u16) -> Self {
        match &mut self.layout {
            ContainerLayout::VStack { padding: p, .. } => *p = padding,
            ContainerLayout::HStack { padding: p, .. } => *p = padding,
        }
        self
    }

    /// Add a child node
    pub fn child(mut self, node: WidgetNode) -> Self {
        self.children.push(node);
        self
    }

    /// Add a leaf widget child
    pub fn leaf(mut self, id: impl Into<String>, widget: impl TreeWidget + 'static) -> Self {
        self.children.push(WidgetNode::leaf(id, widget));
        self
    }

    /// Build the container node
    pub fn build(self) -> WidgetNode {
        WidgetNode::Container {
            id: self.id,
            layout: self.layout,
            children: self.children,
            chrome: self.chrome,
            handler: self.handler,
        }
    }
}

/// The main widget tree container
pub struct WidgetTree {
    root: WidgetNode,
    theme: Theme,
    /// Path to currently focused widget (e.g., ["dialog", "buttons", "ok"])
    focus_path: Vec<String>,
}

impl WidgetTree {
    /// Create with a specific theme
    pub fn with_theme(root: WidgetNode, theme: Theme) -> Self {
        Self {
            root,
            theme,
            focus_path: Vec::new(),
        }
    }

    /// Get a reference to the theme
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Draw the entire tree
    pub fn draw(&self, screen: &mut Screen, bounds: Rect) {
        self.root.draw(screen, bounds, &self.theme);
    }

    /// Handle an event
    pub fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        // Handle Tab/ShiftTab for focus navigation
        match event {
            InputEvent::Tab => {
                self.focus_next();
                return EventResult::Consumed;
            }
            InputEvent::ShiftTab => {
                self.focus_prev();
                return EventResult::Consumed;
            }
            _ => {}
        }

        let target_path = match mouse_position(event) {
            Some((row, col)) => self.root.find_path_at(row, col, bounds),
            None => {
                if self.focus_path.is_empty() {
                    None
                } else {
                    Some(self.focus_path.clone())
                }
            }
        };

        if let Some(path) = target_path.as_ref() {
            if matches!(event, InputEvent::MouseClick { .. }) {
                let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
                if let Some(widget) = self.root.get_widget(&path_refs) {
                    if widget.focusable() {
                        self.set_focus(&path_refs);
                    }
                }
            }
        }

        match target_path {
            Some(path) => self.root.dispatch_event(event, bounds, &path, 0),
            None => EventResult::Ignored,
        }
    }

    /// Get a widget by path
    pub fn get_widget(&self, path: &[&str]) -> Option<&dyn TreeWidget> {
        self.root.get_widget(path)
    }

    /// Get a mutable widget by path
    pub fn get_widget_mut(&mut self, path: &[&str]) -> Option<&mut dyn TreeWidget> {
        self.root.get_widget_mut(path)
    }

    /// Move focus to the next focusable widget
    pub fn focus_next(&mut self) {
        let paths = self.root.collect_focusable(&[]);
        if paths.is_empty() {
            return;
        }

        // Find current focus index
        let current_idx = paths.iter().position(|p| *p == self.focus_path);

        // Move to next (or first if at end)
        let next_idx = match current_idx {
            Some(idx) => (idx + 1) % paths.len(),
            None => 0,
        };

        // Update focus
        self.update_focus(current_idx, next_idx, &paths);
    }

    /// Move focus to the previous focusable widget
    pub fn focus_prev(&mut self) {
        let paths = self.root.collect_focusable(&[]);
        if paths.is_empty() {
            return;
        }

        // Find current focus index
        let current_idx = paths.iter().position(|p| *p == self.focus_path);

        // Move to previous (or last if at beginning)
        let prev_idx = match current_idx {
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => paths.len() - 1,
            None => paths.len() - 1,
        };

        // Update focus
        self.update_focus(current_idx, prev_idx, &paths);
    }

    /// Helper to update focus between widgets
    fn update_focus(&mut self, old_idx: Option<usize>, new_idx: usize, paths: &[Vec<String>]) {
        // Unfocus old widget
        if let Some(idx) = old_idx {
            if let Some(path) = paths.get(idx) {
                let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
                if let Some(widget) = self.root.get_widget_mut(&path_refs) {
                    widget.set_focus(false);
                }
            }
        }

        // Focus new widget
        if let Some(new_path) = paths.get(new_idx) {
            let path_refs: Vec<&str> = new_path.iter().map(|s| s.as_str()).collect();
            if let Some(widget) = self.root.get_widget_mut(&path_refs) {
                widget.set_focus(true);
            }
            self.focus_path = new_path.clone();
        }
    }

    /// Set focus on a specific widget path
    pub fn set_focus(&mut self, path: &[&str]) {
        // Unfocus current
        if !self.focus_path.is_empty() {
            let old_path: Vec<&str> = self.focus_path.iter().map(|s| s.as_str()).collect();
            if let Some(widget) = self.root.get_widget_mut(&old_path) {
                widget.set_focus(false);
            }
        }

        // Focus new
        if let Some(widget) = self.root.get_widget_mut(path) {
            widget.set_focus(true);
        }
        self.focus_path = path.iter().map(|s| s.to_string()).collect();
    }

    /// Get the current focus path
    pub fn focus_path(&self) -> &[String] {
        &self.focus_path
    }
}

/// Widget trait for use in widget trees
///
/// This extends the basic Widget concept with theme support and size hints
/// for the layout engine.
pub trait TreeWidget: Any {
    /// Draw the widget using theme colors
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme);

    /// Handle an input event
    fn handle_event(&mut self, event: &InputEvent, bounds: Rect, phase: EventPhase) -> EventResult;

    /// Whether this widget wants a tight (fixed) width using its minimum size hint.
    fn wants_tight_width(&self) -> bool {
        false
    }

    /// Whether this widget wants full-bleed (extend to container edges ignoring padding).
    /// Useful for separators like HRule that need to connect with container borders.
    fn wants_full_bleed(&self) -> bool {
        false
    }

    /// Downcast support
    fn as_any(&self) -> &dyn Any;

    /// Downcast support (mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get size hint for layout engine
    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }

    /// Whether this widget can receive keyboard focus
    fn focusable(&self) -> bool {
        false
    }

    /// Set focus state
    fn set_focus(&mut self, _focused: bool) {}
}
