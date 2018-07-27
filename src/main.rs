extern crate xcompress;

use xcompress::*;

fn main() {
    let config = Config::new();

    match config {
        Ok(config) => {
            println!("{:?}", config);
        }
        Err(s) => {
            eprintln!("{}", s);
        }
    }
}
