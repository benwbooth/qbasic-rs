//! Event routing and propagation for the widget system

use crate::input::InputEvent;
use super::layout::{Rect, ComputedLayout};
use super::widget::{Widget, EventResult, mouse_position};

/// Routes events to widgets based on computed layout
pub struct EventRouter<'a> {
    layout: &'a ComputedLayout,
}

impl<'a> EventRouter<'a> {
    pub fn new(layout: &'a ComputedLayout) -> Self {
        Self { layout }
    }

    /// Find which widget id contains the given position
    pub fn hit_test(&self, row: u16, col: u16) -> Option<&str> {
        // Find the smallest (most specific) rect containing the point
        let mut best_match: Option<(&str, u32)> = None;

        for (id, rect) in &self.layout.rects {
            if rect.contains(row, col) {
                let area = rect.width as u32 * rect.height as u32;
                match best_match {
                    None => best_match = Some((id.as_str(), area)),
                    Some((_, best_area)) if area < best_area => {
                        best_match = Some((id.as_str(), area));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(id, _)| id)
    }

    /// Route a mouse event to the appropriate widget
    ///
    /// Returns the widget id that should handle the event
    pub fn route_mouse_event(&self, event: &InputEvent) -> Option<&str> {
        mouse_position(event).and_then(|(row, col)| self.hit_test(row, col))
    }

    /// Get bounds for a widget by id
    pub fn bounds(&self, id: &str) -> Option<Rect> {
        self.layout.get(id)
    }
}

/// A container that manages widgets and routes events to them
pub struct WidgetContainer {
    widgets: std::collections::HashMap<String, Box<dyn Widget>>,
    focus_order: Vec<String>,
    focused_index: Option<usize>,
}

impl WidgetContainer {
    pub fn new() -> Self {
        Self {
            widgets: std::collections::HashMap::new(),
            focus_order: Vec::new(),
            focused_index: None,
        }
    }

    /// Register a widget with an id
    pub fn register(&mut self, id: impl Into<String>, widget: Box<dyn Widget>) {
        let id = id.into();
        if widget.focusable() {
            self.focus_order.push(id.clone());
        }
        self.widgets.insert(id, widget);
    }

    /// Get a widget by id
    pub fn get(&self, id: &str) -> Option<&Box<dyn Widget>> {
        self.widgets.get(id)
    }

    /// Get a mutable widget by id
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Widget>> {
        self.widgets.get_mut(id)
    }

    /// Move focus to the next focusable widget
    pub fn focus_next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        // Clear current focus
        if let Some(idx) = self.focused_index {
            if let Some(id) = self.focus_order.get(idx) {
                if let Some(widget) = self.widgets.get_mut(id) {
                    widget.set_focus(false);
                }
            }
        }

        // Move to next
        let next = match self.focused_index {
            None => 0,
            Some(idx) => (idx + 1) % self.focus_order.len(),
        };

        self.focused_index = Some(next);

        // Set new focus
        if let Some(id) = self.focus_order.get(next) {
            if let Some(widget) = self.widgets.get_mut(id) {
                widget.set_focus(true);
            }
        }
    }

    /// Move focus to the previous focusable widget
    pub fn focus_prev(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        // Clear current focus
        if let Some(idx) = self.focused_index {
            if let Some(id) = self.focus_order.get(idx) {
                if let Some(widget) = self.widgets.get_mut(id) {
                    widget.set_focus(false);
                }
            }
        }

        // Move to previous
        let prev = match self.focused_index {
            None => self.focus_order.len() - 1,
            Some(0) => self.focus_order.len() - 1,
            Some(idx) => idx - 1,
        };

        self.focused_index = Some(prev);

        // Set new focus
        if let Some(id) = self.focus_order.get(prev) {
            if let Some(widget) = self.widgets.get_mut(id) {
                widget.set_focus(true);
            }
        }
    }

    /// Focus a specific widget by id
    pub fn focus(&mut self, id: &str) {
        // Clear current focus
        if let Some(idx) = self.focused_index {
            if let Some(focus_id) = self.focus_order.get(idx) {
                if let Some(widget) = self.widgets.get_mut(focus_id) {
                    widget.set_focus(false);
                }
            }
        }

        // Find and set new focus
        if let Some(idx) = self.focus_order.iter().position(|x| x == id) {
            self.focused_index = Some(idx);
            if let Some(widget) = self.widgets.get_mut(id) {
                widget.set_focus(true);
            }
        }
    }

    /// Get the currently focused widget id
    pub fn focused_id(&self) -> Option<&str> {
        self.focused_index
            .and_then(|idx| self.focus_order.get(idx))
            .map(|s| s.as_str())
    }

    /// Handle an event using the computed layout for routing
    ///
    /// For mouse events, routes to the widget under the cursor.
    /// For keyboard events, sends to the focused widget.
    pub fn handle_event(&mut self, event: &InputEvent, layout: &ComputedLayout) -> EventResult {
        let router = EventRouter::new(layout);

        // Check if it's a mouse event
        if let Some((row, col)) = mouse_position(event) {
            // Route to widget under cursor
            if let Some(id) = router.hit_test(row, col) {
                if let Some(bounds) = layout.get(id) {
                    if let Some(widget) = self.widgets.get_mut(id) {
                        return widget.handle_event(event, bounds);
                    }
                }
            }
            return EventResult::Ignored;
        }

        // Keyboard event - send to focused widget
        // Clone the id to avoid borrow conflict
        let focused_id = self.focused_index
            .and_then(|idx| self.focus_order.get(idx))
            .cloned();

        if let Some(id) = focused_id {
            if let Some(bounds) = layout.get(&id) {
                if let Some(widget) = self.widgets.get_mut(&id) {
                    return widget.handle_event(event, bounds);
                }
            }
        }

        EventResult::Ignored
    }

    /// Draw all widgets
    pub fn draw_all(&self, screen: &mut crate::screen::Screen, layout: &ComputedLayout) {
        for (id, widget) in &self.widgets {
            if let Some(bounds) = layout.get(id) {
                widget.draw(screen, bounds);
            }
        }
    }
}

impl Default for WidgetContainer {
    fn default() -> Self {
        Self::new()
    }
}
