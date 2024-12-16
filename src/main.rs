use std::io;
use std::io::Write;

use ansi_strip::NonEsc;

fn main() {
    let reader = io::stdin();
    let mut writer = io::stdout();

    for input in reader.lines() {
        if let Ok(line) = input {
            writer
                .write_all(line.as_str().non_esc().collect::<String>().as_bytes())
                .unwrap();
            writer.write(&[b'\n']).expect("Failed to write to stdout");
        } else {
            eprintln!("Error reading input");
            break;
        }
    }
}

