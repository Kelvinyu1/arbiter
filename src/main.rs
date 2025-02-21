use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
use std::fs;
use std::io::{self, Read, Write};
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

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
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

fn main() {
    let mut path_stack = vec![]; // Store directories as a stack
    let mut current_path = join_path(&path_stack);
    let mut directory = get_directory_contents(&current_path);
    let mut selected = 0;

    set_raw_mode(true);

    loop {
        clear_screen();
        println!("Current Directory: {}\n", current_path);
        println!(
            "Use Arrow Keys (↑ ↓) to navigate, Enter to open, 'b' to go back, and 'q' to quit:\n"
        );

        for (i, option) in directory.iter().enumerate() {
            if i == selected {
                println!("> \x1B[32m{}\x1B[0m", option);
            } else {
                println!("  {}", option);
            }
        }

        let mut input = [0; 1];
        io::stdin().read_exact(&mut input).unwrap();

        match input[0] {
            27 => {
                let mut seq = [0; 2];
                io::stdin().read_exact(&mut seq).unwrap();
                match seq {
                    [91, 65] => {
                        selected = selected.saturating_sub(1);
                    }
                    [91, 66] => {
                        selected = (selected + 1).min(directory.len().saturating_sub(1));
                    }
                    _ => {}
                }
            }
            10 => {
                // Enter key
                let selected_item = &directory[selected];
                let new_path = format!("{}/{}", current_path, selected_item);

                if fs::metadata(&new_path).map(|m| m.is_dir()).unwrap_or(false) {
                    path_stack.push(selected_item.clone()); // Move forward in directory tree
                    current_path = join_path(&path_stack);
                    directory = get_directory_contents(&current_path);
                    selected = 0;
                }
            }
            b'b' => {
                // Back key
                if !path_stack.is_empty() {
                    path_stack.pop(); // Go back one level
                    current_path = join_path(&path_stack);
                    directory = get_directory_contents(&current_path);
                    selected = 0;
                }
            }
            b'q' => {
                // Quit key
                set_raw_mode(false);
                break;
            }
            _ => {}
        }
    }
}
