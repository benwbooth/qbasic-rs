//! Help dialog.

use crate::help;
use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::terminal::Color;
use crate::ui::editor::{tokenize_line, TokenKind};
use crate::ui::floating_window::FloatingWindow;
use crate::ui::layout::{compute_layout, ComputedLayout, LayoutItem, Size};
use crate::ui::scrollbar::{self, ScrollbarColors, ScrollbarState};
use crate::ui::window_chrome;

use super::{DialogContext, DialogController, DialogResult};

const HELP_DEFAULT_WIDTH: u16 = 80;
const HELP_DEFAULT_HEIGHT: u16 = 25;
const HELP_MIN_WIDTH: u16 = 30;
const HELP_MIN_HEIGHT: u16 = 10;

fn help_dialog_layout() -> LayoutItem {
    LayoutItem::vstack(vec![
        LayoutItem::hstack(vec![
            LayoutItem::leaf("title_bar").width(Size::Flex(1)),
            LayoutItem::leaf("maximize").fixed_width(3),
            LayoutItem::leaf("right_border").fixed_width(2),
        ]).fixed_height(1),
        LayoutItem::hstack(vec![
            LayoutItem::spacer().fixed_width(1),
            LayoutItem::leaf("content").width(Size::Flex(1)).height(Size::Flex(1)),
            LayoutItem::leaf("vscrollbar").fixed_width(1).height(Size::Flex(1)),
        ]).height(Size::Flex(1)),
        LayoutItem::hstack(vec![
            LayoutItem::spacer().fixed_width(1),
            LayoutItem::leaf("hscrollbar").width(Size::Flex(1)),
            LayoutItem::leaf("corner").fixed_width(1),
        ]).fixed_height(1),
        LayoutItem::hstack(vec![
            LayoutItem::spacer().fixed_width(1),
            LayoutItem::leaf("nav_bar").width(Size::Flex(1)),
        ]).fixed_height(1),
        LayoutItem::hstack(vec![
            LayoutItem::spacer(),
            LayoutItem::leaf("resize_handle").fixed_width(2),
        ]).fixed_height(1),
    ])
}

pub struct HelpDialog {
    help: help::HelpSystem,
    window: FloatingWindow,
    screen_size: (u16, u16),
    layout: Option<ComputedLayout>,
    vscroll_dragging: bool,
    hscroll_dragging: bool,
    open: bool,
    topic: String,
}

impl HelpDialog {
    pub fn new() -> Self {
        let mut help = help::HelpSystem::new();
        help.load_help_files();
        let window = FloatingWindow::new("Help")
            .with_size(HELP_DEFAULT_WIDTH, HELP_DEFAULT_HEIGHT)
            .with_min_size(HELP_MIN_WIDTH, HELP_MIN_HEIGHT)
            .with_maximize_insets(1, 2, 1, 1);
        Self {
            help,
            window,
            screen_size: (80, 25),
            layout: None,
            vscroll_dragging: false,
            hscroll_dragging: false,
            open: false,
            topic: String::new(),
        }
    }

    /// Set the topic to display (call before open)
    pub fn set_topic(&mut self, topic: String) {
        self.topic = topic;
    }

    fn sync_topic(&mut self) {
        let normalized = self.topic.to_lowercase().replace(' ', "-");
        if self.help.current_topic != normalized {
            self.help.navigate_to(&self.topic);
        }
    }

    fn ensure_window_active(&mut self) {
        let (sw, sh) = self.screen_size;
        self.window.center(sw, sh);
    }
}

impl DialogController for HelpDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.layout = None;
        self.vscroll_dragging = false;
        self.hscroll_dragging = false;
        self.sync_topic();
        self.ensure_window_active();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.open = false;
        self.layout = None;
    }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.screen_size = (width, height);
        if self.open && self.window.is_maximized() {
            self.window.apply_maximize(width, height);
        }
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open {
            return;
        }

        self.sync_topic();

        let bounds = self.window.bounds();
        let x = bounds.x;
        let y = bounds.y;
        let width = bounds.width;
        let height = bounds.height;
        let layout = compute_layout(&help_dialog_layout(), bounds);

        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::LightGray, Color::Black);
        screen.draw_box(y, x, width, height, Color::LightGray, Color::Black);

        let title = self.help.current_document()
            .map(|d| d.title.clone())
            .unwrap_or_else(|| "Help".to_string());
        self.window.title = title.clone();

        if let Some(rect) = layout.get("title_bar") {
            let title_str = format!(" {} ", title);
            let title_x = rect.x + (rect.width.saturating_sub(title_str.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, &title_str, Color::Cyan, Color::Black);
        }

        window_chrome::draw_maximize_button(screen, y, x, width, self.window.is_maximized(), Color::LightGray, Color::Black);

        if let Some(content_rect) = layout.get("content") {
            let content_width = content_rect.width as usize;
            let content_height = content_rect.height as usize;
            let (lines, links, styles, max_width) = self.help.render(content_width);

            let max_vscroll = lines.len().saturating_sub(1);
            if self.help.scroll > max_vscroll {
                self.help.scroll = max_vscroll;
            }
            let max_hscroll = max_width.saturating_sub(1);
            if self.help.scroll_col > max_hscroll {
                self.help.scroll_col = max_hscroll;
            }

            let scroll = self.help.scroll;
            let scroll_col = self.help.scroll_col;
            let selected_link = self.help.selected_link;

            for (i, line) in lines.iter().skip(scroll).take(content_height).enumerate() {
                let row = content_rect.y + i as u16;
                let col = content_rect.x;
                let line_idx = scroll + i;

                let line_links: Vec<_> = links.iter().enumerate()
                    .filter(|(_, link)| link.line == line_idx)
                    .collect();
                let line_styles: Vec<_> = styles.iter().filter(|s| s.line == line_idx).collect();
                let is_code_block = line_styles.iter().any(|s| s.style == help::TextStyle::CodeBlock);

                if is_code_block && line_links.is_empty() {
                    let tokens = tokenize_line(line);
                    let mut x_pos = 0usize;
                    for token in tokens {
                        let token_fg = match token.kind {
                            TokenKind::Keyword => Color::White,
                            TokenKind::String => Color::LightMagenta,
                            TokenKind::Number => Color::LightCyan,
                            TokenKind::Comment => Color::LightGray,
                            TokenKind::Operator => Color::LightGreen,
                            TokenKind::Identifier => Color::Yellow,
                            TokenKind::Punctuation => Color::White,
                            TokenKind::Whitespace => Color::Yellow,
                        };
                        for ch in token.text.chars() {
                            if x_pos >= scroll_col && x_pos - scroll_col < content_width {
                                screen.set(row, col + (x_pos - scroll_col) as u16, ch, token_fg, Color::Black);
                            }
                            x_pos += 1;
                        }
                    }
                    for screen_pos in x_pos.saturating_sub(scroll_col)..content_width {
                        screen.set(row, col + screen_pos as u16, ' ', Color::Yellow, Color::Black);
                    }
                } else {
                    let chars: Vec<char> = line.chars().collect();
                    for screen_pos in 0..content_width {
                        let char_pos = scroll_col + screen_pos;
                        let ch = chars.get(char_pos).copied().unwrap_or(' ');
                        let mut in_link = false;
                        let mut is_selected = false;
                        for (link_idx, link) in &line_links {
                            if char_pos >= link.col_start && char_pos < link.col_end {
                                in_link = true;
                                is_selected = *link_idx == selected_link;
                                break;
                            }
                        }
                        let mut style = None;
                        for s in &line_styles {
                            if char_pos >= s.col_start && char_pos < s.col_end {
                                style = Some(s.style);
                                break;
                            }
                        }
                        let (fg, bg) = if in_link {
                            if is_selected { (Color::White, Color::Cyan) } else { (Color::Green, Color::Black) }
                        } else {
                            match style {
                                Some(help::TextStyle::Code) | Some(help::TextStyle::CodeBlock) => (Color::Yellow, Color::Black),
                                Some(help::TextStyle::Bold) => (Color::White, Color::Black),
                                Some(help::TextStyle::Italic) => (Color::Cyan, Color::Black),
                                None => (Color::LightGray, Color::Black),
                            }
                        };
                        screen.set(row, col + screen_pos as u16, ch, fg, bg);
                    }
                }
            }

            if let Some(vscroll_rect) = layout.get("vscrollbar") {
                let vstate = ScrollbarState::new(scroll, lines.len(), 1);
                scrollbar::draw_vertical(screen, vscroll_rect.x, vscroll_rect.y, vscroll_rect.y + vscroll_rect.height.saturating_sub(1), &vstate, &ScrollbarColors::dark());
            }
            if let Some(hscroll_rect) = layout.get("hscrollbar") {
                let hstate = ScrollbarState::new(scroll_col, max_width, 1);
                scrollbar::draw_horizontal(screen, hscroll_rect.y, hscroll_rect.x, hscroll_rect.x + hscroll_rect.width.saturating_sub(1), &hstate, &ScrollbarColors::dark());
            }
        }

        if let Some(nav_rect) = layout.get("nav_bar") {
            let nav_hint = if self.help.link_count() > 0 {
                "Tab:Link  Enter:Follow  Backspace:Back  Esc:Close"
            } else {
                "Arrows:Scroll  Backspace:Back  Esc:Close"
            };
            screen.write_str(nav_rect.y, nav_rect.x, nav_hint, Color::Cyan, Color::Black);
        }

        self.layout = Some(layout);
    }

    fn handle_event(&mut self, event: &InputEvent, _ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }

        self.sync_topic();
        let (sw, sh) = self.screen_size;

        if matches!(event, InputEvent::MouseRelease { .. }) {
            self.vscroll_dragging = false;
            self.hscroll_dragging = false;
        }

        if let InputEvent::MouseClick { row, col } = event {
            if !self.window.bounds().contains(*row, *col) {
                return DialogResult::Closed;
            }
        }

        if self.window.handle_event_with_screen(event, sw, sh) {
            return DialogResult::Open;
        }

        match event {
            InputEvent::Escape => return DialogResult::Closed,
            InputEvent::Enter => {
                if let Some(link) = self.help.selected_link().cloned() {
                    self.help.navigate_to(&link.target);
                }
            }
            InputEvent::Backspace => {
                if !self.help.go_back() {
                    return DialogResult::Closed;
                }
            }
            InputEvent::Tab => {
                let count = self.help.link_count();
                if count > 0 {
                    self.help.selected_link = (self.help.selected_link + 1) % count;
                }
            }
            InputEvent::ShiftTab => {
                let count = self.help.link_count();
                if count > 0 {
                    self.help.selected_link = if self.help.selected_link == 0 { count - 1 } else { self.help.selected_link - 1 };
                }
            }
            InputEvent::CursorUp => { if self.help.scroll > 0 { self.help.scroll -= 1; } }
            InputEvent::CursorDown => { self.help.scroll += 1; }
            InputEvent::CursorLeft => { if self.help.scroll_col > 0 { self.help.scroll_col -= 1; } }
            InputEvent::CursorRight => { self.help.scroll_col += 1; }
            InputEvent::PageUp => { self.help.scroll = self.help.scroll.saturating_sub(10); }
            InputEvent::PageDown => { self.help.scroll += 10; }
            InputEvent::Home => { self.help.scroll = 0; self.help.scroll_col = 0; }
            InputEvent::End => { self.help.scroll = usize::MAX / 2; }
            InputEvent::ScrollUp { .. } => { self.help.scroll = self.help.scroll.saturating_sub(3); }
            InputEvent::ScrollDown { .. } => { self.help.scroll += 3; }
            _ => {}
        }

        DialogResult::Open
    }
}
