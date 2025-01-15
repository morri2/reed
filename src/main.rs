use core::fmt;
use std::{
    fmt::write,
    io::{self, stdout, Read, Stdout},
    time::{Duration, SystemTime},
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    prelude::{Backend, CrosstermBackend},
    style::{Style, Stylize},
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
    let _read_res = file.read_to_string(&mut text);

    let mut app: App = App {
        text: text.chars().collect(),
        word_start: 0,
        word_end: 0,
        exit: false,

        pause: false,
        ms_per_word: 150,
        word_speed_factor: 100,
        show_context_when_playing: false,
    };

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).expect("failed to init new termial.");
    _ = app.run(&mut terminal);
}

#[derive(Debug)]
pub struct App {
    text: Vec<char>,
    word_start: usize,
    word_end: usize,
    ms_per_word: u32, // in ms
    pause: bool,
    exit: bool,
    word_speed_factor: u32, // factor in percent
    show_context_when_playing: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum CharCategory {
    Glyph,
    WhiteSpace,
    Punctuation,
}

pub fn get_category(ch: char) -> CharCategory {
    return match ch {
        ' ' | '\n' | '\t' => CharCategory::WhiteSpace,
        '.' | ':' | ';' | ',' | '-' => CharCategory::Punctuation,
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

pub fn sanitize_string(mut s: String) -> String {
    s = s.replace("\r\n", "\n");
    s = s.replace("\n", " ");
    return s;
}

impl App {
    pub fn get_word_string(&self, cidx_start: usize, cidx_end: usize) -> String {
        return sanitize_string(self.text[cidx_start..cidx_end].iter().collect());
    }

    pub fn get_word_string_forw_padded(&self, cidx_start: usize, len: usize) -> String {
        let mut string: String = String::new();

        string.push_str(
            &self.text[cidx_start..(cidx_start + len).min(self.text.len())]
                .iter()
                .collect::<String>(),
        );

        // padding
        if cidx_start + len > self.text.len() {
            for _ in 0..(cidx_start + len - self.text.len()) {
                string.push(' ');
            }
        }

        return sanitize_string(string);
    }

    pub fn get_word_string_back_padded(&self, cidx_end: usize, len: usize) -> String {
        let mut string: String = String::new();

        // padding
        if len > cidx_end {
            for _ in 0..(len - cidx_end) {
                string.push(' ');
            }
        }

        string.push_str(
            &self.text[(cidx_end - len.min(cidx_end))..cidx_end]
                .iter()
                .collect::<String>(),
        );

        return sanitize_string(string);
    }

    /// returns true when done
    pub fn next_word(&mut self) -> bool {
        self.word_start = self.word_end;
        while self.word_start < self.text.len()
            && get_category(self.text[self.word_start]) == CharCategory::WhiteSpace
        {
            self.word_start += 1;
        }

        self.word_end = self.word_start + 1;

        while self.word_end < self.text.len()
            && get_category(self.text[self.word_end]) != CharCategory::WhiteSpace
        {
            self.word_end += 1;
        }

        return self.word_end >= self.text.len();
    }

    /// returns true if reached start
    pub fn prev_word(&mut self) -> bool {
        self.word_end = self.word_start;

        while self.word_end > 0
            && get_category(self.text[self.word_end - 1]) == CharCategory::WhiteSpace
        {
            self.word_end -= 1;
        }

        self.word_start = self.word_end;

        while self.word_start > 0
            && get_category(self.text[self.word_start - 1]) != CharCategory::WhiteSpace
        {
            self.word_start -= 1;
        }
        return self.word_start > 0;
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut timer: SystemTime = SystemTime::now();

        let cursor_end_pos = { terminal.backend_mut().get_cursor_position()? };

        while !self.exit {
            if timer.elapsed().expect("timer error").as_millis() as u32
                > ((self.ms_per_word * self.word_speed_factor) / 100)
                && !self.pause
            {
                if !self.pause {
                    self.exit = self.next_word();
                    self.word_speed_factor = 100;
                }

                timer = SystemTime::now();
            }

            // pause loop

            terminal.draw(|frame| self.draw(frame))?;

            _ = terminal.backend_mut().set_cursor_position(cursor_end_pos);

            if let Ok(true) = event::poll(Duration::from_millis(10)) {
                match event::read()? {
                    Event::Key(key_event) => {
                        if key_event.kind == KeyEventKind::Press {
                            timer = SystemTime::now(); // reset timer on any input

                            match key_event.code {
                                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('p') => {
                                    self.pause = !self.pause;
                                    timer = SystemTime::now();
                                    print!("> Pause {}               ", self.pause);
                                }
                                KeyCode::Char('h') => print!(
                                    "[h]elp: [p]ause [c]tx [q]uit [r]eset [^/v]speed [</>]move  "
                                ),
                                KeyCode::Char('c') => {
                                    self.show_context_when_playing = !self.show_context_when_playing
                                }
                                KeyCode::Char('q') => {
                                    self.exit = true;
                                }
                                KeyCode::Char('r') => {
                                    self.pause = true;
                                    self.word_start = 0;
                                    self.word_end = 0;
                                    _ = self.next_word();
                                }
                                KeyCode::Left => _ = self.prev_word(),
                                KeyCode::Right => self.exit = self.next_word(),

                                KeyCode::Down => {
                                    if self.ms_per_word < 1000 {
                                        self.ms_per_word += 25
                                    }
                                }
                                KeyCode::Up => {
                                    if self.ms_per_word > 25 {
                                        self.ms_per_word -= 25
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Event::FocusLost => self.pause = true,
                    _ => {}
                }
            }
        }
        println!("done!");
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let width = frame.area().width / 2;
        let height = 5;
        // let area = frame.area();
        let area = Rect {
            x: frame.area().width / 4,
            y: frame.area().height - 6,
            width,
            height,
        };
        clear_lines(frame, frame.area().height - height, frame.area().height);

        let buf = frame.buffer_mut();

        let mut ctx_len = (width - 4 - (self.word_end - self.word_start) as u16) as usize / 2;

        if !self.pause && !self.show_context_when_playing {
            ctx_len = 0;
        }

        Paragraph::new(Line::from(vec![
            self.get_word_string_back_padded(self.word_start, ctx_len)
                .blue(),
            //" ".into(),
            self.get_word_string(self.word_start, self.word_end).into(),
            //" ".into(),
            self.get_word_string_forw_padded(self.word_end, ctx_len)
                .blue(),
        ]))
        .centered()
        .block(
            Block::bordered()
                .padding(Padding::vertical(1))
                .title(format!(" {}wpm ", 60_000 / (self.ms_per_word))),
        )
        .render(area, buf);
    }
}

pub fn clear_lines(frame: &mut Frame, low: u16, high: u16) {
    let frame_area = frame.area();
    let line = (0..frame_area.width).map(|_| ' ').collect::<String>();

    for y in low..high {
        frame.buffer_mut().set_string(0, y, &line, Style::default());
    }
}

pub fn clear(frame: &mut Frame, area: Rect) {
    let line = (0..area.width).map(|_| ' ').collect::<String>();

    for y in area.y..(area.y + area.height) {
        frame
            .buffer_mut()
            .set_string(area.x, y, &line, Style::default());
    }
}
