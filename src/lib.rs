use std::fs;
use std::error::Error;

mod parser;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(&config.filename)?;

    let _result = parser::parse_obj(content);

    Ok(())
}

pub struct Config {
    filename: String
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }

        Ok(Config {
            filename: args[1].clone(),
        })
    }
}