use std::env::Args;
use std::error::Error;
use std::fs;

mod parser;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(&config.filename)?;

    let _result = parser::parse_obj_threaded(content)?;

    Ok(())
}

pub struct Config {
    filename: String,
}

impl Config {
    pub fn new(mut args: Args) -> Result<Config, &'static str> {
        args.next();

        let filename = match args.next() {
            Some(arg) => arg,
            None => return Err("Filename not specified."),
        };

        Ok(Config { filename })
    }
}
