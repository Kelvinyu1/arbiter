use libc::{tcgetattr, tcsetattr, termios, winsize, ECHO, ICANON, TCSANOW, TIOCGWINSZ};
use std::fs;
use std::io::{self, Read, Write};
use std::mem::zeroed;
use std::os::unix::io::AsRawFd;

fn set_raw_mode(enable: bool) {
    unsafe {
        let fd = io::stdin().as_raw_fd();
        let mut termios: termios = std::mem::zeroed();
        tcgetattr(fd, &mut termios);

        if enable {
            termios.c_lflag &= !(ICANON | ECHO);
        } else {
            termios.c_lflag |= ICANON | ECHO;
        }

        tcsetattr(fd, TCSANOW, &termios);
    }
}

fn get_terminal_size() -> (usize, usize) {
    unsafe {
        let mut size: winsize = zeroed();
        if libc::ioctl(0, TIOCGWINSZ, &mut size) == 0 {
            return (size.ws_col as usize, size.ws_row as usize);
        }
    }
    (80, 24) // Default size if detection fails
}

fn draw_border(width: usize, height: usize, offset_x: usize, offset_y: usize) {
    let horizontal = "─".repeat(width - 2);
    print!("\x1B[{};{}H┌{}┐", offset_y, offset_x, horizontal); // Top border
    for i in 1..height - 1 {
        print!(
            "\x1B[{};{}H│\x1B[{};{}H│",
            offset_y + i,
            offset_x,
            offset_y + i,
            offset_x + width - 1
        ); // Side borders
    }
    print!(
        "\x1B[{};{}H└{}┘",
        offset_y + height - 1,
        offset_x,
        horizontal
    ); // Bottom border
}

// fn clear_screen() {
//     print!("\x1B[H\x1B[J"); // Move cursor to top-left and clear screen
//     io::stdout().flush().unwrap();
// }
fn clear_screen() -> io::Result<()> {
    print!("\x1B[H\x1B[J"); // Move cursor to top-left and clear screen
    io::stdout().flush()?;
    Ok(())
}
fn get_directory_contents(path: &str) -> Vec<String> {
    let mut contents = vec![];
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_name = entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| "Invalid UTF-8".to_string());
            contents.push(entry_name);
        }
    }
    contents
}

fn join_path(path_stack: &[String]) -> String {
    if path_stack.is_empty() {
        return "./".to_string();
    }
    format!("./{}", path_stack.join("/"))
}

fn main() -> io::Result<()> {
    let mut path_stack = vec![];
    let mut current_path = join_path(&path_stack);
    let mut directory = get_directory_contents(&current_path);
    let mut selected = 0;

    set_raw_mode(true);

    loop {
        let (term_width, term_height) = get_terminal_size();
        let ui_width = 60; // Slightly wider
        let ui_height = 18; // Increased height for spacing
        let offset_x = (term_width.saturating_sub(ui_width)) / 2;
        let offset_y = (term_height.saturating_sub(ui_height)) / 2;

        print!("\x1B[?25l"); // Hide cursor
        clear_screen()?;
        draw_border(ui_width, ui_height, offset_x, offset_y);

        // Set content start positions inside the border
        let content_start_y = offset_y + 1;
        let content_start_x = offset_x + 2;
        let max_text_width = ui_width - 4; // Ensure text fits inside

        // Print directory path
        print!(
            "\x1B[{};{}HCurrent Directory: {}",
            content_start_y,
            content_start_x,
            &current_path
                .chars()
                .take(max_text_width)
                .collect::<String>()
        );

        // Print instructions neatly inside the border
        let instructions_start_y = content_start_y + 1;
        let instructions = [
            "(↑ ↓)  Navigate",
            "Enter  Open Directory",
            "'b'    Back",
            "'r'    Rename",
            "'q'    Quit",
        ];

        for (i, instr) in instructions.iter().enumerate() {
            print!(
                "\x1B[{};{}H{}",
                instructions_start_y + i,
                content_start_x,
                instr
            );
        }

        // Print directory listing
        let list_start_y = instructions_start_y + instructions.len() + 1;
        for (i, option) in directory.iter().enumerate() {
            let y_pos = list_start_y + i;
            if y_pos >= offset_y + ui_height - 1 {
                break; // Prevents overflowing past the border
            }

            let truncated_option = option.chars().take(max_text_width - 4).collect::<String>(); // Prevents overflow
            if i == selected {
                print!(
                    "\x1B[{};{}H> \x1B[32m{}\x1B[0m",
                    y_pos, content_start_x, truncated_option
                );
            } else {
                print!("\x1B[{};{}H  {}", y_pos, content_start_x, truncated_option);
            }
        }
        // Correct cursor position (2 lines under last directory entry)
        let last_list_y = list_start_y + directory.len().saturating_sub(1);
        let cursor_y = last_list_y + 2;
        let cursor_x = offset_x + 2;

        print!("\x1B[{};{}H", cursor_y, cursor_x); // Move cursor down
        print!("\x1B[?25h"); // Show cursor back
        io::stdout().flush().unwrap();

        let mut input = [0; 1];
        io::stdin().read_exact(&mut input).unwrap();

        match input[0] {
            27 => {
                let mut seq = [0; 2];
                io::stdin().read_exact(&mut seq).unwrap();
                match seq {
                    [91, 65] => selected = selected.saturating_sub(1), // Up arrow
                    [91, 66] => selected = (selected + 1).min(directory.len().saturating_sub(1)), // Down arrow
                    _ => {}
                }
            }
            10 => {
                let selected_item = &directory[selected];
                let new_path = format!("{}/{}", current_path, selected_item);

                if fs::metadata(&new_path).map(|m| m.is_dir()).unwrap_or(false) {
                    path_stack.push(selected_item.clone());
                    current_path = join_path(&path_stack);
                    directory = get_directory_contents(&current_path);
                    selected = 0;
                }
            }
            b'b' => {
                if !path_stack.is_empty() {
                    path_stack.pop();
                    current_path = join_path(&path_stack);
                    directory = get_directory_contents(&current_path);
                    selected = 0;
                }
            }
            b'r' => {
                set_raw_mode(false);
                print!("Enter new name for {}: ", directory[selected]);
                io::stdout().flush().unwrap();

                let mut new_name = String::new();
                io::stdin().read_line(&mut new_name).unwrap();
                let new_name = new_name.trim();

                if !new_name.is_empty() {
                    let old_path = format!("{}/{}", current_path, directory[selected]);
                    let new_path = format!("{}/{}", current_path, new_name);

                    if let Err(e) = fs::rename(&old_path, &new_path) {
                        println!("Failed to rename: {}", e);
                    } else {
                        directory = get_directory_contents(&current_path);
                        selected = selected.min(directory.len().saturating_sub(1));
                    }
                }
                set_raw_mode(true);
            }
            b'q' => {
                set_raw_mode(false);
                print!("\x1B[?25h"); // Restore cursor visibility before exiting
                clear_screen()?;
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
