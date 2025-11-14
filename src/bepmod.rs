use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BepinexMod {
    pub namespace: String,
    pub name: String,
    pub from: Option<String>,
    pub to: Option<String>,
}