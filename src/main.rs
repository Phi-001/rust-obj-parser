use std::env;
use std::time::Instant;
use std::process;

use rust_obj_parser::Config;

fn main() {
    let now = Instant::now();

    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(err) = rust_obj_parser::run(config) {
        println!("application error: {}", err);
        process::exit(1);
    }

    let ms = (now.elapsed().as_nanos() as f64) / 1000000f64;

    println!("took {} milliseconds", ms);
}