use std::env;
use std::process;
use std::time::Instant;

use rust_obj_parser::Config;

use rust_obj_parser::parser;
use std::fs;

fn main() {
    // let now = Instant::now();

    // let config = Config::new(env::args()).unwrap_or_else(|err| {
    //     println!("Problem parsing arguments: {}", err);
    //     process::exit(1);
    // });

    // if let Err(err) = rust_obj_parser::run(config) {
    //     println!("Application error: {}", err);
    //     process::exit(1);
    // }

    // let ms = (now.elapsed().as_nanos() as f64) / 1000000f64;

    // println!("Took {} milliseconds", ms);
    let content = fs::read_to_string("al.obj").unwrap();
    for _ in 0..1000 {
        parser::parse_obj_threaded(content.clone()).unwrap();
    }
}
