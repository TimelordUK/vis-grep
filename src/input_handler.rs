use eframe::egui;
use log::info;

#[derive(Debug, Clone)]
pub enum NavigationCommand {
    NextMatch,
    PreviousMatch,
    FirstMatch,
    LastMatch,
    NextMatchWithCount(usize),
    PreviousMatchWithCount(usize),

    // File-level navigation
    FirstMatchInCurrentFile,      // ^ - jump to first match in current file
    LastMatchInCurrentFile,       // $ - jump to last match in current file
    NextFile,                     // N - jump to first match in next file
    PreviousFile,                 // P - jump to first match in previous file
    NextFileWithCount(usize),     // 2N - jump forward 2 files
    PreviousFileWithCount(usize), // 2P - jump backward 2 files

    // Clipboard operations
    YankMatchedLine, // yy - yank (copy) matched line to clipboard

    // File operations
    OpenInExplorer, // gf - open file in explorer/finder

    // Bookmarks/Markers
    SetMark(char),  // ma, mb, etc - set a mark
    GotoMark(char), // 'a, 'b, etc - go to a mark
}

pub struct InputHandler {
    // State for building up multi-key commands (like "gg" or "3n")
    pending_keys: String,
    count_buffer: String,
    waiting_for_mark_char: bool,      // True when waiting for 'a' in 'ma'
    waiting_for_goto_mark_char: bool, // True when waiting for 'a' in "'a"
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            pending_keys: String::new(),
            count_buffer: String::new(),
            waiting_for_mark_char: false,
            waiting_for_goto_mark_char: false,
        }
    }

    /// Process keyboard input and return a command if one is complete
    pub fn process_input(&mut self, ctx: &egui::Context) -> Option<NavigationCommand> {
        let mut command = None;

        ctx.input(|i| {
            // Check for special shift+number combos FIRST (before digit processing)
            // '^' - first match in current file (Shift+6)
            if i.key_pressed(egui::Key::Num6)
                && i.modifiers.shift
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                info!("Command: ^ (first match in current file)");
                command = Some(NavigationCommand::FirstMatchInCurrentFile);
                self.reset();
                return;
            }
            // '$' - last match in current file (Shift+4)
            if i.key_pressed(egui::Key::Num4)
                && i.modifiers.shift
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                info!("Command: $ (last match in current file)");
                command = Some(NavigationCommand::LastMatchInCurrentFile);
                self.reset();
                return;
            }

            // 'y' - start of yank sequence (yy = yank matched line)
            if i.key_pressed(egui::Key::Y)
                && !i.modifiers.shift
                && !i.modifiers.ctrl
                && !i.modifiers.alt
            {
                if self.pending_keys == "y" {
                    // Second 'y' - yank matched line
                    info!("Command: yy (yank matched line)");
                    command = Some(NavigationCommand::YankMatchedLine);
                    self.reset();
                    return;
                } else {
                    // First 'y' - wait for second key
                    self.pending_keys = "y".to_string();
                    info!("Pending: y (waiting for second y)");
                    return;
                }
            }

            // Check for digit keys to build up count (e.g., "3n" -> move 3 times)
            // Only process if shift is NOT pressed (to avoid conflicts with ^ and $)
            for key in &[
                egui::Key::Num0,
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ] {
                if i.key_pressed(*key)
                    && !i.modifiers.shift
                    && !i.modifiers.ctrl
                    && !i.modifiers.alt
                {
                    let digit = match key {
                        egui::Key::Num0 => '0',
                        egui::Key::Num1 => '1',
                        egui::Key::Num2 => '2',
                        egui::Key::Num3 => '3',
                        egui::Key::Num4 => '4',
                        egui::Key::Num5 => '5',
                        egui::Key::Num6 => '6',
                        egui::Key::Num7 => '7',
                        egui::Key::Num8 => '8',
                        egui::Key::Num9 => '9',
                        _ => unreachable!(),
                    };

                    // Don't allow leading zeros
                    if !(self.count_buffer.is_empty() && digit == '0') {
                        self.count_buffer.push(digit);
                        info!("Count buffer: {}", self.count_buffer);
                    }
                    return; // Exit early after processing digit
                }
            }

            // 'n' - next match (with optional count)
            if i.key_pressed(egui::Key::N) && !i.modifiers.ctrl && !i.modifiers.alt {
                if i.modifiers.shift {
                    // Shift+N - next file
                    command = if self.count_buffer.is_empty() {
                        Some(NavigationCommand::NextFile)
                    } else {
                        let count = self.count_buffer.parse::<usize>().unwrap_or(1);
                        info!("Next file with count: {}", count);
                        Some(NavigationCommand::NextFileWithCount(count))
                    };
                } else {
                    // lowercase n - next match
                    command = if self.count_buffer.is_empty() {
                        Some(NavigationCommand::NextMatch)
                    } else {
                        let count = self.count_buffer.parse::<usize>().unwrap_or(1);
                        info!("Next match with count: {}", count);
                        Some(NavigationCommand::NextMatchWithCount(count))
                    };
                }
                self.reset();
            }
            // 'p' - previous match (with optional count)
            else if i.key_pressed(egui::Key::P) && !i.modifiers.ctrl && !i.modifiers.alt {
                if i.modifiers.shift {
                    // Shift+P - previous file
                    command = if self.count_buffer.is_empty() {
                        Some(NavigationCommand::PreviousFile)
                    } else {
                        let count = self.count_buffer.parse::<usize>().unwrap_or(1);
                        info!("Previous file with count: {}", count);
                        Some(NavigationCommand::PreviousFileWithCount(count))
                    };
                } else {
                    // lowercase p - previous match
                    command = if self.count_buffer.is_empty() {
                        Some(NavigationCommand::PreviousMatch)
                    } else {
                        let count = self.count_buffer.parse::<usize>().unwrap_or(1);
                        info!("Previous match with count: {}", count);
                        Some(NavigationCommand::PreviousMatchWithCount(count))
                    };
                }
                self.reset();
            }
            // 'g' - start of multi-key sequence (gg = first match, gf = open in explorer)
            else if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl && !i.modifiers.alt {
                if self.pending_keys == "g" {
                    // Second 'g' - go to first match
                    info!("Command: gg (first match)");
                    command = Some(NavigationCommand::FirstMatch);
                    self.reset();
                } else if i.modifiers.shift {
                    // Shift+G - go to last match
                    info!("Command: G (last match)");
                    command = Some(NavigationCommand::LastMatch);
                    self.reset();
                } else {
                    // First 'g' - wait for second key
                    self.pending_keys = "g".to_string();
                    info!("Pending: g (waiting for second g or f)");
                }
            }
            // 'f' - could be part of 'gf' sequence
            else if i.key_pressed(egui::Key::F)
                && !i.modifiers.ctrl
                && !i.modifiers.alt
                && !i.modifiers.shift
            {
                if self.pending_keys == "g" {
                    // 'gf' - open file in explorer
                    info!("Command: gf (open in explorer)");
                    command = Some(NavigationCommand::OpenInExplorer);
                    self.reset();
                } else {
                    // 'f' without 'g' prefix - ignore for now
                    info!("Ignoring standalone 'f'");
                }
            }
            // 'm' - start mark sequence (ma, mb, etc)
            else if i.key_pressed(egui::Key::M)
                && !i.modifiers.ctrl
                && !i.modifiers.alt
                && !i.modifiers.shift
            {
                self.waiting_for_mark_char = true;
                self.pending_keys = "m".to_string();
                info!("Pending: m (waiting for mark letter)");
            }
            // "'" (apostrophe/quote) - start goto mark sequence ('a, 'b, etc)
            else if i.key_pressed(egui::Key::Quote)
                && !i.modifiers.ctrl
                && !i.modifiers.alt
                && !i.modifiers.shift
            {
                self.waiting_for_goto_mark_char = true;
                self.pending_keys = "'".to_string();
                info!("Pending: ' (waiting for mark letter)");
            }
            // Letter keys - could be mark character
            else if self.waiting_for_mark_char || self.waiting_for_goto_mark_char {
                // Check for any letter a-z
                let mark_char = Self::get_letter_from_key(i);
                if let Some(ch) = mark_char {
                    if self.waiting_for_mark_char {
                        info!("Command: m{} (set mark)", ch);
                        command = Some(NavigationCommand::SetMark(ch));
                    } else {
                        info!("Command: '{} (goto mark)", ch);
                        command = Some(NavigationCommand::GotoMark(ch));
                    }
                    self.reset();
                }
            }
            // Escape to cancel pending commands
            else if i.key_pressed(egui::Key::Escape) {
                if !self.pending_keys.is_empty() || !self.count_buffer.is_empty() {
                    info!("Cancelled pending command");
                    self.reset();
                }
            }
        });

        command
    }

    fn reset(&mut self) {
        self.pending_keys.clear();
        self.count_buffer.clear();
        self.waiting_for_mark_char = false;
        self.waiting_for_goto_mark_char = false;
    }

    /// Get the current pending input state for display (e.g., "3" or "g")
    pub fn get_status(&self) -> String {
        if !self.count_buffer.is_empty() || !self.pending_keys.is_empty() {
            format!("{}{}", self.count_buffer, self.pending_keys)
        } else {
            String::new()
        }
    }

    /// Try to extract a letter (a-z) from the current key press
    fn get_letter_from_key(input: &egui::InputState) -> Option<char> {
        for (key, ch) in &[
            (egui::Key::A, 'a'),
            (egui::Key::B, 'b'),
            (egui::Key::C, 'c'),
            (egui::Key::D, 'd'),
            (egui::Key::E, 'e'),
            (egui::Key::F, 'f'),
            (egui::Key::G, 'g'),
            (egui::Key::H, 'h'),
            (egui::Key::I, 'i'),
            (egui::Key::J, 'j'),
            (egui::Key::K, 'k'),
            (egui::Key::L, 'l'),
            (egui::Key::M, 'm'),
            (egui::Key::N, 'n'),
            (egui::Key::O, 'o'),
            (egui::Key::P, 'p'),
            (egui::Key::Q, 'q'),
            (egui::Key::R, 'r'),
            (egui::Key::S, 's'),
            (egui::Key::T, 't'),
            (egui::Key::U, 'u'),
            (egui::Key::V, 'v'),
            (egui::Key::W, 'w'),
            (egui::Key::X, 'x'),
            (egui::Key::Y, 'y'),
            (egui::Key::Z, 'z'),
        ] {
            if input.key_pressed(*key)
                && !input.modifiers.ctrl
                && !input.modifiers.alt
                && !input.modifiers.shift
            {
                return Some(*ch);
            }
        }
        None
    }
}
