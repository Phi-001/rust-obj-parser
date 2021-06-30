use std::env;
use std::fs;
use std::time::Instant;

mod parser;

fn main() {
    let now = Instant::now();

    let args: Vec<String> = env::args().collect();
    let filename = parse_args(&args);

    let content = fs::read_to_string(&filename)
        .expect("something went worong. Maybe your file doesn't exist?");

    let result = parser::parse_obj(content);

    let ms = (now.elapsed().as_nanos() as f64) / 1000000f64;

    println!("{:?}", result.position);
    println!("took {} milliseconds", ms);
}

fn parse_args(args: &[String]) -> &str {
    let filename = &args[1];

    filename
}