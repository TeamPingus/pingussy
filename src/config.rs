use hocon::HoconLoader;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;

pub fn create_config() -> std::io::Result<()> {
    let mut file = File::create("stuff.conf")?;
    file.write_all(b"{ token: TOKEN }")?;
    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub token: String,
}

lazy_static! {
    pub static ref CONFIG: Config = get_config();
}

fn get_config() -> Config {
    let configs: Config = HoconLoader::new()
        .load_file("./stuff.conf")
        .expect("Config load err")
        .resolve()
        .expect("Config deserialize err");

    configs
}
