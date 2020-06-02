#[macro_use]
extern crate log;

use std::env;
use curl::easy::Easy;
use std::io::{stdout, Write};
use std::time::Instant;

pub fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let url = env::args().nth(1).unwrap();
    let mut easy = Easy::new();
    easy.url(&url).unwrap();
    easy.write_function(|data| {
        stdout().write_all(data).unwrap();
        Ok(data.len())
    }).unwrap();
    info!("Sending request to {}", url);
    let time = Instant::now();
    easy.perform().unwrap();
    info!("Time: {:#?}", time.elapsed());
}
