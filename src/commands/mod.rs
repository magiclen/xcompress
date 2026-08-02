mod compression;
mod decompression;
mod pipeline;

use std::{fs, io, io::Write, num::NonZeroUsize, path::Path, thread};

use anyhow::anyhow;
pub use compression::*;
pub use decompression::*;
use pipeline::*;

#[inline]
fn try_delete_file<P: AsRef<Path>>(file_path: P) {
    let _ = fs::remove_file(file_path);
}

#[inline]
fn cpu_count(single_thread: bool) -> usize {
    if single_thread { 1 } else { thread::available_parallelism().map_or(1, NonZeroUsize::get) }
}

fn read_password(password: Option<String>) -> anyhow::Result<String> {
    match password {
        Some(password) => {
            if password.is_empty() {
                print!("Password (visible): ");
                io::stdout().flush()?;

                let mut password = String::new();

                if io::stdin().read_line(&mut password)? == 0 {
                    return Err(anyhow!("Stdin is closed."));
                }

                // only the line terminator can be trimmed, because a password may end with spaces
                let length = password.trim_end_matches(['\n', '\r']).len();

                password.truncate(length);

                Ok(password)
            } else {
                Ok(password)
            }
        },
        None => Ok(String::new()),
    }
}
