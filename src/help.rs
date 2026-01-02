//! Help system with markdown parsing and navigation
#![allow(dead_code)]

use std::collections::HashMap;

/// A parsed help document
#[derive(Clone, Debug)]
pub struct HelpDocument {
    pub title: String,
    pub elements: Vec<HelpElement>,
    pub links: Vec<HelpLink>,
}

/// An element in a help document
#[derive(Clone, Debug)]
pub enum HelpElement {
    /// Heading (level 1-6)
    Heading(u8, String),
    /// Plain paragraph text
    Paragraph(String),
    /// Code block
    Code(String),
    /// Table (headers, rows)
    Table(Vec<String>, Vec<Vec<String>>),
    /// Blank line
    Blank,
}

/// A hyperlink in the document
#[derive(Clone, Debug)]
pub struct HelpLink {
    /// Line number (0-indexed in rendered output)
    pub line: usize,
    /// Column start
    pub col_start: usize,
    /// Column end
    pub col_end: usize,
    /// Display text
    pub text: String,
    /// Target topic
    pub target: String,
}

/// Text styling type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextStyle {
    Bold,
    Italic,
    Code,
    CodeBlock,
}

/// A styled span in the document
#[derive(Clone, Debug)]
pub struct StyleSpan {
    /// Line number (0-indexed in rendered output)
    pub line: usize,
    /// Column start
    pub col_start: usize,
    /// Column end
    pub col_end: usize,
    /// Style to apply
    pub style: TextStyle,
}

/// Help system state
pub struct HelpSystem {
    /// Loaded help documents
    documents: HashMap<String, HelpDocument>,
    /// Navigation history
    pub history: Vec<String>,
    /// Current topic
    pub current_topic: String,
    /// Vertical scroll position
    pub scroll: usize,
    /// Horizontal scroll position
    pub scroll_col: usize,
    /// Selected link index
    pub selected_link: usize,
    /// Rendered lines cache (topic, lines, links, styles, max_line_width)
    rendered_cache: Option<(String, Vec<String>, Vec<HelpLink>, Vec<StyleSpan>, usize)>,
}

impl HelpSystem {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            history: Vec::new(),
            current_topic: String::new(),
            scroll: 0,
            scroll_col: 0,
            selected_link: 0,
            rendered_cache: None,
        }
    }

    /// Load all help files from the help directory
    pub fn load_help_files(&mut self) {
        // Try to load from executable directory first, then current directory
        let help_dirs = [
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("help"))),
            Some(std::path::PathBuf::from("help")),
        ];

        for help_dir in help_dirs.iter().flatten() {
            if help_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(help_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.extension().map(|e| e == "md").unwrap_or(false) {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let topic = path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                self.documents.insert(topic, parse_markdown(&content));
                            }
                        }
                    }
                }
                break; // Use first directory that exists
            }
        }

        // Add embedded fallback content
        self.add_embedded_content();
    }

    /// Add embedded help content as fallback
    fn add_embedded_content(&mut self) {
        // Only add if not already loaded from files
        if !self.documents.contains_key("survival-guide") {
            self.documents.insert("survival-guide".to_string(), parse_markdown(SURVIVAL_GUIDE_MD));
        }
        if !self.documents.contains_key("index") {
            self.documents.insert("index".to_string(), parse_markdown(INDEX_MD));
        }
        if !self.documents.contains_key("shortcuts") {
            self.documents.insert("shortcuts".to_string(), parse_markdown(SHORTCUTS_MD));
        }
    }

    /// Navigate to a topic
    pub fn navigate_to(&mut self, topic: &str) {
        let topic_lower = topic.to_lowercase().replace(' ', "-");
        if !self.current_topic.is_empty() {
            self.history.push(self.current_topic.clone());
        }
        self.current_topic = topic_lower;
        self.scroll = 0;
        self.scroll_col = 0;
        self.selected_link = 0;
        self.rendered_cache = None;
    }

    /// Go back to previous topic
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.current_topic = prev;
            self.scroll = 0;
            self.scroll_col = 0;
            self.selected_link = 0;
            self.rendered_cache = None;
            true
        } else {
            false
        }
    }

    /// Get the current document
    pub fn current_document(&self) -> Option<&HelpDocument> {
        self.documents.get(&self.current_topic)
            .or_else(|| self.documents.get("index"))
    }

    /// Render current document to lines and collect links and styles
    /// Returns (lines, links, styles, max_line_width)
    pub fn render(&mut self, width: usize) -> (Vec<String>, Vec<HelpLink>, Vec<StyleSpan>, usize) {
        // Check cache
        if let Some((ref cached_topic, ref lines, ref links, ref styles, max_width)) = self.rendered_cache {
            if cached_topic == &self.current_topic {
                return (lines.clone(), links.clone(), styles.clone(), max_width);
            }
        }

        let doc = match self.current_document() {
            Some(d) => d.clone(),
            None => return (vec!["Help topic not found.".to_string()], vec![], vec![], 25),
        };

        let mut lines = Vec::new();
        let mut links = Vec::new();
        let mut styles = Vec::new();

        for element in &doc.elements {
            match element {
                HelpElement::Heading(level, text) => {
                    if *level == 1 {
                        // Title - centered with decoration
                        lines.push(String::new());
                        let padding = (width.saturating_sub(text.len())) / 2;
                        lines.push(format!("{:>pad$}{}", "", text, pad = padding));
                        lines.push(format!("{:>pad$}{}", "", "=".repeat(text.len().min(width)), pad = padding));
                        lines.push(String::new());
                    } else {
                        // Subheading
                        lines.push(String::new());
                        lines.push(text.clone());
                        lines.push("-".repeat(text.len().min(width)));
                    }
                }
                HelpElement::Paragraph(text) => {
                    // Parse links and render with styles
                    let (rendered, para_links, para_styles) = render_paragraph_with_links(text, lines.len(), width);
                    for line in rendered {
                        lines.push(line);
                    }
                    links.extend(para_links);
                    styles.extend(para_styles);
                }
                HelpElement::Code(code) => {
                    // Add code block with styling
                    let code_start_line = lines.len();
                    for (i, line) in code.lines().enumerate() {
                        let formatted = format!("  {}", line);
                        styles.push(StyleSpan {
                            line: code_start_line + i,
                            col_start: 0,
                            col_end: formatted.len(),
                            style: TextStyle::CodeBlock,
                        });
                        lines.push(formatted);
                    }
                    lines.push(String::new());
                }
                HelpElement::Table(headers, rows) => {
                    // Calculate column widths
                    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
                    for row in rows {
                        for (i, cell) in row.iter().enumerate() {
                            if i < col_widths.len() {
                                col_widths[i] = col_widths[i].max(cell.len());
                            }
                        }
                    }

                    // Render header
                    let header_line: String = headers.iter().enumerate()
                        .map(|(i, h)| format!("{:width$}", h, width = col_widths.get(i).copied().unwrap_or(10) + 2))
                        .collect::<Vec<_>>()
                        .join("");
                    lines.push(header_line);

                    // Separator
                    let sep_line: String = col_widths.iter()
                        .map(|w| format!("{:-<width$}  ", "", width = *w))
                        .collect();
                    lines.push(sep_line);

                    // Rows
                    for row in rows {
                        let row_line: String = row.iter().enumerate()
                            .map(|(i, cell)| format!("{:width$}", cell, width = col_widths.get(i).copied().unwrap_or(10) + 2))
                            .collect::<Vec<_>>()
                            .join("");
                        lines.push(row_line);
                    }
                    lines.push(String::new());
                }
                HelpElement::Blank => {
                    lines.push(String::new());
                }
            }
        }

        // Calculate max line width for horizontal scrolling
        let max_width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        self.rendered_cache = Some((self.current_topic.clone(), lines.clone(), links.clone(), styles.clone(), max_width));
        (lines, links, styles, max_width)
    }

    /// Get link at selected index
    pub fn selected_link(&self) -> Option<&HelpLink> {
        if let Some((_, _, ref links, _, _)) = self.rendered_cache {
            links.get(self.selected_link)
        } else {
            None
        }
    }

    /// Get total number of links
    pub fn link_count(&self) -> usize {
        if let Some((_, _, ref links, _, _)) = self.rendered_cache {
            links.len()
        } else {
            0
        }
    }
}

impl Default for HelpSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse markdown content into a HelpDocument
fn parse_markdown(content: &str) -> HelpDocument {
    let mut elements = Vec::new();
    let mut title = String::new();
    let mut in_code_block = false;
    let mut code_buffer = String::new();
    let mut in_table = false;
    let mut table_headers: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut para_buffer = String::new();

    for line in content.lines() {
        // Code block handling
        if line.starts_with("```") {
            if in_code_block {
                elements.push(HelpElement::Code(code_buffer.clone()));
                code_buffer.clear();
                in_code_block = false;
            } else {
                // Flush paragraph
                if !para_buffer.is_empty() {
                    elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
                    para_buffer.clear();
                }
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !code_buffer.is_empty() {
                code_buffer.push('\n');
            }
            code_buffer.push_str(line);
            continue;
        }

        // Table handling
        if line.contains('|') && !line.starts_with("```") {
            // Flush paragraph
            if !para_buffer.is_empty() {
                elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
                para_buffer.clear();
            }

            let cells: Vec<String> = line.split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')) {
                // This is the separator line, skip it
                continue;
            }

            if !in_table {
                table_headers = cells;
                in_table = true;
            } else {
                table_rows.push(cells);
            }
            continue;
        } else if in_table {
            // End of table
            elements.push(HelpElement::Table(table_headers.clone(), table_rows.clone()));
            table_headers.clear();
            table_rows.clear();
            in_table = false;
        }

        // Heading
        if line.starts_with('#') {
            // Flush paragraph
            if !para_buffer.is_empty() {
                elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
                para_buffer.clear();
            }

            let level = line.chars().take_while(|c| *c == '#').count() as u8;
            let text = line.trim_start_matches('#').trim().to_string();
            if level == 1 && title.is_empty() {
                title = text.clone();
            }
            elements.push(HelpElement::Heading(level, text));
            continue;
        }

        // Blank line
        if line.trim().is_empty() {
            if !para_buffer.is_empty() {
                elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
                para_buffer.clear();
            }
            elements.push(HelpElement::Blank);
            continue;
        }

        // List item - each bullet is its own line
        if line.starts_with("- ") {
            // Flush any pending paragraph first
            if !para_buffer.is_empty() {
                elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
                para_buffer.clear();
            }
            // Add bullet as its own paragraph
            elements.push(HelpElement::Paragraph(format!("  • {}", &line[2..])));
            continue;
        }

        // Accumulate paragraph
        if !para_buffer.is_empty() {
            para_buffer.push(' ');
        }
        para_buffer.push_str(line);
    }

    // Flush remaining content
    if in_table && !table_headers.is_empty() {
        elements.push(HelpElement::Table(table_headers, table_rows));
    }
    if !para_buffer.is_empty() {
        elements.push(HelpElement::Paragraph(para_buffer.trim().to_string()));
    }

    HelpDocument {
        title: if title.is_empty() { "Help".to_string() } else { title },
        elements,
        links: Vec::new(), // Links are extracted during rendering
    }
}

/// Render a paragraph with markdown links, returning lines, link positions, and styles
fn render_paragraph_with_links(text: &str, start_line: usize, width: usize) -> (Vec<String>, Vec<HelpLink>, Vec<StyleSpan>) {
    let mut result_lines = Vec::new();
    let mut links = Vec::new();
    let mut styles = Vec::new();
    let mut current_line = String::new();
    let mut line_num = start_line;
    let mut col = 0;

    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        // Check for markdown link [text](target)
        if chars[i] == '[' {
            let mut j = i + 1;
            let mut link_text = String::new();
            while j < chars.len() && chars[j] != ']' {
                link_text.push(chars[j]);
                j += 1;
            }
            if j + 1 < chars.len() && chars[j] == ']' && chars[j + 1] == '(' {
                let mut k = j + 2;
                let mut target = String::new();
                while k < chars.len() && chars[k] != ')' {
                    target.push(chars[k]);
                    k += 1;
                }
                if k < chars.len() && chars[k] == ')' {
                    let display = format!("◄{}►", link_text);

                    if col + display.chars().count() > width && !current_line.is_empty() {
                        result_lines.push(current_line);
                        current_line = String::new();
                        line_num += 1;
                        col = 0;
                    }

                    links.push(HelpLink {
                        line: line_num,
                        col_start: col,
                        col_end: col + display.chars().count(),
                        text: link_text.clone(),
                        target,
                    });

                    current_line.push_str(&display);
                    col += display.chars().count();
                    i = k + 1;
                    continue;
                }
            }
        }

        // Check for inline code `text`
        if chars[i] == '`' && (i + 1 >= chars.len() || chars[i + 1] != '`') {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '`' {
                j += 1;
            }
            if j < chars.len() {
                let code_start_col = col;
                for k in (i + 1)..j {
                    if col >= width {
                        result_lines.push(current_line);
                        current_line = String::new();
                        line_num += 1;
                        col = 0;
                    }
                    current_line.push(chars[k]);
                    col += 1;
                }
                styles.push(StyleSpan {
                    line: line_num,
                    col_start: code_start_col,
                    col_end: col,
                    style: TextStyle::Code,
                });
                i = j + 1;
                continue;
            }
        }

        // Check for bold **text**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            let mut j = i + 2;
            while j + 1 < chars.len() && !(chars[j] == '*' && chars[j + 1] == '*') {
                j += 1;
            }
            if j + 1 < chars.len() {
                let bold_start_col = col;
                for k in (i + 2)..j {
                    if col >= width {
                        result_lines.push(current_line);
                        current_line = String::new();
                        line_num += 1;
                        col = 0;
                    }
                    current_line.push(chars[k]);
                    col += 1;
                }
                styles.push(StyleSpan {
                    line: line_num,
                    col_start: bold_start_col,
                    col_end: col,
                    style: TextStyle::Bold,
                });
                i = j + 2;
                continue;
            }
        }

        // Check for italic *text* or _text_
        if (chars[i] == '*' || chars[i] == '_') && (i + 1 < chars.len() && chars[i + 1] != chars[i]) {
            let delim = chars[i];
            let mut j = i + 1;
            while j < chars.len() && chars[j] != delim {
                j += 1;
            }
            if j < chars.len() && j > i + 1 {
                let italic_start_col = col;
                for k in (i + 1)..j {
                    if col >= width {
                        result_lines.push(current_line);
                        current_line = String::new();
                        line_num += 1;
                        col = 0;
                    }
                    current_line.push(chars[k]);
                    col += 1;
                }
                styles.push(StyleSpan {
                    line: line_num,
                    col_start: italic_start_col,
                    col_end: col,
                    style: TextStyle::Italic,
                });
                i = j + 1;
                continue;
            }
        }

        // Regular character - word wrap at spaces
        if col >= width && chars[i] == ' ' {
            result_lines.push(current_line);
            current_line = String::new();
            line_num += 1;
            col = 0;
            i += 1;
            continue;
        }

        if col >= width {
            result_lines.push(current_line);
            current_line = String::new();
            line_num += 1;
            col = 0;
        }

        current_line.push(chars[i]);
        col += 1;
        i += 1;
    }

    if !current_line.is_empty() {
        result_lines.push(current_line);
    }

    (result_lines, links, styles)
}

// Embedded help content as fallback
const SURVIVAL_GUIDE_MD: &str = r#"# QBasic Survival Guide

A quick reference for the most essential commands.

## Getting Around

| Action | Key |
|--------|-----|
| Access menus | Alt + letter |
| Help on item | F1 |
| Switch windows | F6 |

## Running Programs

| Action | Key |
|--------|-----|
| Run | F5 |
| Step | F8 |
| Breakpoint | F9 |
| Stop | Ctrl+C |

## See Also
- [Index](index)
- [Keyboard Shortcuts](shortcuts)
"#;

const INDEX_MD: &str = r#"# Help Index

## Getting Started
- [Survival Guide](survival-guide)
- [Keyboard Shortcuts](shortcuts)

## Statements
- [PRINT](print) - Display output
- [INPUT](input) - Read input
- [IF...THEN](if) - Conditions
- [FOR...NEXT](for) - Loops

Press Escape to close help.
"#;

const SHORTCUTS_MD: &str = r#"# Keyboard Shortcuts

## Navigation
| Key | Action |
|-----|--------|
| Arrow keys | Move cursor |
| Home/End | Start/end of line |
| Ctrl+Home/End | Start/end of file |

## Editing
| Key | Action |
|-----|--------|
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+X | Cut |
| Ctrl+Z | Undo |

## Running
| Key | Action |
|-----|--------|
| F5 | Run |
| F8 | Step |
| F9 | Breakpoint |

## See Also
- [Survival Guide](survival-guide)
"#;
