mod compression;
mod decompression;
mod pipeline;
mod staging;
mod validation;

use std::{io, io::IsTerminal, num::NonZeroUsize, thread};

use anyhow::anyhow;
pub use compression::*;
pub use decompression::*;
use pipeline::*;

#[inline]
fn cpu_count(single_thread: bool) -> usize {
    if single_thread { 1 } else { thread::available_parallelism().map_or(1, NonZeroUsize::get) }
}

fn read_password(password: Option<String>) -> anyhow::Result<String> {
    match password {
        Some(password) => {
            if password.is_empty() {
                if io::stdin().is_terminal() {
                    return Ok(rpassword::prompt_password("Password: ")?);
                }

                let mut password = String::new();

                if io::stdin().read_line(&mut password)? == 0 {
                    return Err(anyhow!("Stdin is closed."));
                }

                trim_line_ending(&mut password);

                Ok(password)
            } else {
                Ok(password)
            }
        },
        None => Ok(String::new()),
    }
}

fn trim_line_ending(line: &mut String) {
    let length = line.trim_end_matches(['\n', '\r']).len();

    line.truncate(length);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ending_is_removed_from_password() {
        let mut password = String::from("password  \r\n");

        trim_line_ending(&mut password);

        assert_eq!("password  ", password);
    }
}
