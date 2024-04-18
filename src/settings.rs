use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BepinexMod {
    pub url: String,
    pub mapping: Vec<Vec<String>>,
}

#[allow(dead_code)]
impl BepinexMod {
    pub fn new(url: String, mapping: Vec<Vec<String>>) -> Self {
        Self {
            url: url,
            mapping: mapping,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bepinex {
    pub url: String,
    pub mapping: Vec<String>,
}

#[allow(dead_code)]
impl Bepinex {
    pub fn new(url: String, mapping: Vec<String>) -> Self {
        Self {
            url: url,
            mapping: mapping,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub bepinex: Bepinex,
    pub mods: Vec<BepinexMod>,
}

#[allow(dead_code)]
impl Settings {
    pub fn new(bepinex: Bepinex, mods: Vec<BepinexMod>) -> Self {
        Self {
            bepinex: bepinex,
            mods: mods,
        }
    }

    pub fn read_from_file(file: &str) -> Result<Settings, std::io::Error> {
        let f = std::fs::File::open(file)?;
        let r = std::io::BufReader::new(f);
        let settings: Settings = serde_json::from_reader(r)?;

        Ok(settings)
    }
}