use anyhow::Result;
use crossterm::cursor::{self, MoveTo, MoveToColumn, MoveToNextLine, MoveToPreviousLine};
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{ExecutableCommand, QueueableCommand};
use regex::Regex;
use std::io::{stdout, Write};

pub struct Suggestion {
    pub text: String,
    pub description: String,
}

fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

pub fn read_input_with_suggestions(prompt: &str, suggestions: &[Suggestion]) -> Result<String> {
    let mut buffer = String::new();
    let mut cursor_pos = 0;
    let mut suggestion_index: Option<usize> = None;
    let mut scroll_offset = 0;
    let mut horizontal_scroll = 0; // For scrolling long input text
    let mut stdout = stdout();

    // Box drawing characters
    let box_top_left = '╭';
    let box_top_right = '╮';
    let box_bottom_left = '╰';
    let box_bottom_right = '╯';
    let box_horizontal = '─';
    let box_vertical = '│';

    terminal::enable_raw_mode()?;

    let mut is_first_render = true;

    loop {
        // 0. Preparation
        let (cols, _) = terminal::size()?;
        let cols = cols as usize;

        // Calculate visual widths
        let prompt_stripped = strip_ansi(prompt);
        let prompt_width = prompt_stripped.chars().count();
        // Box content: "│ " + prompt + buffer + " │"
        // Inner width needed: 1 (space) + prompt + buffer + 1 (space)
        // But we want the box to extend to the right reasonably
        let content_width = prompt_width + buffer.len() + 2;
        let min_box_width = 60;
        let box_width = (content_width + 2) // +2 for borders
            .max(min_box_width)
            .min(cols);

        // 1. Clear Previous Frame
        if !is_first_render {
            // We assume cursor is at the input line (middle of box)
            stdout.execute(MoveToPreviousLine(1))?;
            stdout.execute(MoveToColumn(0))?;
            stdout.execute(Clear(ClearType::FromCursorDown))?;
        }
        is_first_render = false;

        // Capture start position (Top of box)
        let (_, start_row) = cursor::position()?;

        // 2. Render Box
        // Top Border
        let top_border_len = box_width.saturating_sub(2);
        let top_border = format!(
            "{}{}{}",
            box_top_left,
            box_horizontal.to_string().repeat(top_border_len),
            box_top_right
        );
        stdout.execute(Print(top_border))?;
        stdout.execute(MoveToNextLine(1))?;
        stdout.execute(MoveToColumn(0))?;

        // Middle Line (Prompt + Input)
        // Calculate available space for input text
        let available_width = box_width
            .saturating_sub(4) // 2 for borders + 2 for spaces around content
            .saturating_sub(prompt_width);

        // Calculate horizontal scroll to keep cursor visible
        if cursor_pos < horizontal_scroll {
            horizontal_scroll = cursor_pos;
        } else if cursor_pos >= horizontal_scroll + available_width {
            horizontal_scroll = cursor_pos.saturating_sub(available_width) + 1;
        }

        // Get the visible portion of the buffer
        let visible_buffer: String = buffer
            .chars()
            .skip(horizontal_scroll)
            .take(available_width)
            .collect();

        stdout.execute(Print(format!("{} ", box_vertical)))?;
        stdout.execute(Print(prompt))?;
        stdout.execute(Print(&visible_buffer))?;

        // Fill remaining space with spaces
        let current_inner_len = 1 + prompt_width + visible_buffer.len(); // " " + prompt + visible_buffer
        let remaining_space = box_width
            .saturating_sub(2)
            .saturating_sub(current_inner_len);
        if remaining_space > 0 {
            stdout.execute(Print(" ".repeat(remaining_space)))?;
        }
        stdout.execute(Print(format!("{}", box_vertical)))?;

        stdout.execute(MoveToNextLine(1))?;
        stdout.execute(MoveToColumn(0))?;

        // Bottom Border
        let bottom_border = format!(
            "{}{}{}",
            box_bottom_left,
            box_horizontal.to_string().repeat(top_border_len),
            box_bottom_right
        );
        stdout.execute(Print(bottom_border))?;
        stdout.execute(MoveToNextLine(1))?;
        stdout.execute(MoveToColumn(0))?;

        // 3. Render Suggestions
        let filtered_suggestions: Vec<&Suggestion> = if let Some(search) = buffer.strip_prefix('/')
        {
            suggestions
                .iter()
                .filter(|s| s.text.starts_with('/') && s.text[1..].starts_with(search))
                .collect()
        } else {
            Vec::new()
        };

        if !filtered_suggestions.is_empty() {
            // Update selection index wrap-around
            if let Some(idx) = suggestion_index {
                if idx >= filtered_suggestions.len() {
                    suggestion_index = Some(0);
                }
            } else {
                suggestion_index = Some(0);
            }

            let idx = suggestion_index.unwrap();
            let list_height = 8; // Max visible suggestions

            // Adjust scroll
            if idx < scroll_offset {
                scroll_offset = idx;
            } else if idx >= scroll_offset + list_height {
                scroll_offset = idx - list_height + 1;
            }
            // Ensure scroll is valid
            if scroll_offset + list_height > filtered_suggestions.len() {
                scroll_offset = filtered_suggestions.len().saturating_sub(list_height);
            }

            // Render Up Arrow
            if scroll_offset > 0 {
                stdout.queue(MoveToColumn(2))?;
                stdout.queue(SetForegroundColor(Color::DarkGrey))?;
                stdout.queue(Print("▲"))?;
                stdout.queue(ResetColor)?;
                stdout.queue(MoveToNextLine(1))?;
            }

            // Render Items
            let _end_index = (scroll_offset + list_height).min(filtered_suggestions.len());
            for (i, suggestion) in filtered_suggestions
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(list_height)
            {
                stdout.queue(MoveToColumn(2))?; // Indent
                if Some(i) == suggestion_index {
                    stdout.queue(SetForegroundColor(Color::Black))?;
                    stdout.queue(SetBackgroundColor(Color::White))?;
                } else {
                    stdout.queue(SetForegroundColor(Color::Cyan))?;
                    stdout.queue(SetBackgroundColor(Color::Reset))?;
                }

                let text = format!("{}  - {}", suggestion.text, suggestion.description);
                // Truncate if too long (simple check)
                let max_text_len = cols.saturating_sub(4);
                let text_preview = if text.len() > max_text_len {
                    format!("{}...", &text[..max_text_len.saturating_sub(3)])
                } else {
                    text
                };

                stdout.queue(Print(text_preview))?;
                stdout.queue(ResetColor)?;
                stdout.queue(MoveToNextLine(1))?;
            }

            // Render Down Arrow
            if scroll_offset + list_height < filtered_suggestions.len() {
                stdout.queue(MoveToColumn(2))?;
                stdout.queue(SetForegroundColor(Color::DarkGrey))?;
                stdout.queue(Print("▼"))?;
                stdout.queue(ResetColor)?;
            }
        } else {
            suggestion_index = None;
            scroll_offset = 0;
        }

        // 4. Position Cursor
        // We want cursor at Middle Line of box.
        // Start row was Top Line.
        // Middle Line is start_row + 1.
        let cursor_row = start_row + 1;
        // Col: "│ " (2 chars) + prompt_width + (cursor_pos - horizontal_scroll)
        let visible_cursor_pos = cursor_pos.saturating_sub(horizontal_scroll);
        let cursor_col = 2 + prompt_width + visible_cursor_pos;

        stdout.execute(MoveTo(cursor_col as u16, cursor_row))?;
        stdout.flush()?;

        // 5. Handle Input
        if let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = read()?
            && kind == KeyEventKind::Press {
                match code {
                    KeyCode::Enter => {
                        if let Some(idx) = suggestion_index
                            && !filtered_suggestions.is_empty() {
                                buffer = filtered_suggestions[idx].text.clone();
                                // Select and break
                                break;
                            }
                        if !buffer.is_empty() {
                            break;
                        }
                    }
                    KeyCode::Char(c) => {
                        if modifiers == KeyModifiers::CONTROL && c == 'c' {
                            buffer.clear();
                            break;
                        }
                        if cursor_pos < buffer.len() {
                            buffer.insert(cursor_pos, c);
                        } else {
                            buffer.push(c);
                        }
                        cursor_pos += 1;
                    }
                    KeyCode::Backspace => {
                        if cursor_pos > 0 {
                            buffer.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                        }
                    }
                    KeyCode::Left => {
                        cursor_pos = cursor_pos.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        if cursor_pos < buffer.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Up => {
                        if let Some(idx) = suggestion_index {
                            if idx > 0 {
                                suggestion_index = Some(idx - 1);
                            }
                        } else if !filtered_suggestions.is_empty() {
                            // If no selection but list exists, select last?
                            suggestion_index = Some(filtered_suggestions.len() - 1);
                        }
                    }
                    KeyCode::Down => {
                        if let Some(idx) = suggestion_index {
                            if idx < filtered_suggestions.len().saturating_sub(1) {
                                suggestion_index = Some(idx + 1);
                            }
                        } else if !filtered_suggestions.is_empty() {
                            suggestion_index = Some(0);
                        }
                    }
                    KeyCode::Tab => {
                        if let Some(idx) = suggestion_index
                            && !filtered_suggestions.is_empty() {
                                buffer = filtered_suggestions[idx].text.clone();
                                cursor_pos = buffer.len();
                                // Don't break, just fill
                            }
                    }
                    KeyCode::Esc => {
                        suggestion_index = None;
                    }
                    _ => {}
                }
            }
    }

    // Cleanup
    // Move to Top of box and clear down to leave a clean state
    if !is_first_render {
        stdout.execute(MoveToPreviousLine(1))?;
        stdout.execute(MoveToColumn(0))?;
        stdout.execute(Clear(ClearType::FromCursorDown))?;
    }

    // Print final committed line (simple text, no box, for history)
    stdout.execute(Print(format!("{}{}{}", prompt, buffer, '\n')))?;

    terminal::disable_raw_mode()?;
    Ok(buffer)
}
