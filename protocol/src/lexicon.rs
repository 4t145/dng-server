use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Lexicon {
    pub name: String,
    pub lang: String,
    pub author: String,
    pub tags: Vec<String>,
    pub version: String,
    pub brief: String,
    pub lexicon: Vec<String>
}