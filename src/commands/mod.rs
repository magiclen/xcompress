mod compression;
mod decompression;

use std::{fs, io, io::Write, path::Path};

use anyhow::anyhow;
pub use compression::*;
pub use decompression::*;
use execute::generic_array::typenum::U32;
use scanner_rust::Scanner;

#[inline]
fn try_delete_file<P: AsRef<Path>>(file_path: P) {
    if fs::remove_file(file_path).is_err() {}
}
fn read_password(password: Option<String>) -> anyhow::Result<String> {
    match password {
        Some(password) => {
            if password.is_empty() {
                print!("Password (visible): ");
                io::stdout().flush()?;

                let mut sc: Scanner<_, U32> = Scanner::new2(io::stdin());

                sc.next_line()?.ok_or_else(|| anyhow!("Stdin is closed."))
            } else {
                Ok(password)
            }
        },
        None => Ok(String::new()),
    }
}
