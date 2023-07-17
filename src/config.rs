use std::{fs::File, io::Read, path::Path};

use crate::modules::Config as ModuleConfig;
use serde::Deserialize;
use twilight_model::id::{Id, marker::GuildMarker};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub token: String,
    pub db_url: String,
    pub home_guild: Id<GuildMarker>,

    pub modules: Option<ModuleConfig>,
}

pub fn parse_config(path: &Path) -> anyhow::Result<Config> {
    let mut file = File::open(path)?;
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents)?;

    Ok(toml::from_str(&file_contents)?)
}
