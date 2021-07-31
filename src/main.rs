#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

fn main() {
    use std::env;
    use std::process;

    use rust_obj_parser::Config;

    let config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(err) = rust_obj_parser::run(config) {
        println!("Application error: {}", err);
        process::exit(1);
    }

    // use rust_obj_parser::parser;
    // use std::fs;
    // use std::time::Instant;
    // let content = fs::read_to_string("al.obj").unwrap();
    // let now = Instant::now();
    // for _ in 0..1000 {
    //     parser::parse_obj_threaded(content.clone()).unwrap();
    // }
    // println!("{:?}", now.elapsed().as_nanos() as f64 / 1000000f64);
}
