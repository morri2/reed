use core::fmt;
use std::{
    fmt::write,
    io::{self, stdout, Read, Stdout},
    time::{Duration, SystemTime},
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    prelude::{Backend, CrosstermBackend},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Clear, Padding, Paragraph, Widget},
    DefaultTerminal, Frame, Terminal,
};

const WPM_LIST: [u32; 8] = [150, 200, 250, 300, 350, 400, 450, 500];

fn main() {
    print!("\n\n\n\n\n\n\n");

    let mut file = std::fs::File::open("text.txt").unwrap();

    let mut text = String::new();
    let mut read_res = file.read_to_string(&mut text);

    let mut words = text.split_whitespace();

    let mut app: App = App {
        text,
        word_start: 0,
        word_end: 0,
        exit: false,

        pause: false,
        ms_per_word: 150,
    };

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).expect("failed to init new termial.");
    _ = app.run(&mut terminal);
}

#[derive(Debug)]
pub struct App {
    text: String,
    word_start: usize,
    word_end: usize,
    ms_per_word: u32, // in ms
    pause: bool,
    exit: bool,
}
#[derive(Debug, PartialEq, Eq)]
enum CharCategory {
    Glyph,
    WhiteSpace,
}

pub fn get_category(ch: char) -> CharCategory {
    return match ch {
        ' ' | '\n' | '\t' => CharCategory::WhiteSpace,
        _ => CharCategory::Glyph,
    };
}

pub fn char_at(byte_idx: usize, text: &String) -> Option<char> {
    text[byte_idx..].chars().next()
}

/// returns char at position, and the end of that char
pub fn char_at_end(byte_idx: usize, text: &String) -> Option<(usize, char)> {
    let ch = char_at(byte_idx, text)?;
    Some((byte_idx + ch.len_utf8(), ch))
}

impl App {
    /// returns true when done
    pub fn next_word(&mut self) -> bool {
        while let Some((end, ch)) = char_at_end(self.word_start, &self.text) {
            if get_category(ch) == CharCategory::WhiteSpace {
                self.word_start = end;
                continue;
            } else {
                self.word_end = end;
                break;
            }
        }

        if char_at_end(self.word_end, &self.text).is_none() {
            return true;
        }

        while let Some((end, ch)) = char_at_end(self.word_end, &self.text) {
            if get_category(ch) != CharCategory::WhiteSpace {
                self.word_end = end;
                continue;
            } else {
                break;
            }
        }

        return false;
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut words = self.text.split_whitespace();
        let mut word = words.next().unwrap();
        let mut timer: SystemTime = SystemTime::now();

        let cursor_end_pos = { terminal.backend_mut().get_cursor_position()? };

        while !self.exit {
            if timer.elapsed().expect("timer error").as_millis() as u32 > self.ms_per_word {
                word = words.next().unwrap();
                timer = SystemTime::now();
            }

            // pause loop

            terminal.draw(|frame| self.draw(frame, word))?;

            _ = terminal.backend_mut().set_cursor_position(cursor_end_pos);

            if let Ok(true) = event::poll(Duration::from_millis(10)) {
                match event::read()? {
                    Event::Key(key_event) => match key_event.code {
                        KeyCode::Enter => self.pause = !self.pause,
                        KeyCode::Char('g') => print!("hellO!"),
                        _ => {}
                    },
                    Event::FocusLost => self.pause = true,
                    _ => {}
                }
            }

            if !self.pause {
                break;
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame, word: &str) {
        // let area = frame.area();
        let area = Rect {
            x: frame.area().width / 4,
            y: frame.area().height - 6,
            width: frame.area().width / 2,
            height: 5,
        };
        let buf = frame.buffer_mut();

        Clear.render(area, buf);

        Paragraph::new(word)
            .centered()
            .block(
                Block::bordered()
                    .padding(Padding::vertical(1))
                    .title(format!(" {}wpm ", 60_000 / (self.ms_per_word))),
            )
            .render(area, buf);
    }
}
