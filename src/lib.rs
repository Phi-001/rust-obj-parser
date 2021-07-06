use std::env::Args;
use std::error::Error;
use std::fs;

mod parser;

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(&config.filename)?;

    let _result = parser::parse_obj_threaded(content.clone())?;

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

#[test]
fn test() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("al.obj")?;

    let result_1 = parser::parse_obj_threaded(content.clone())?;
    let result_2 = parser::_parse_obj(content.clone())?;

    assert_eq!(result_1, result_2);

    Ok(())
}
