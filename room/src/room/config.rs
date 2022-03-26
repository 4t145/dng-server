use crate::consts::*;
pub struct Config {
    pub round_time: usize,
    pub token: Option<u128>,
    pub lexicon: Lexicon,
    pub lexicon_lock: bool,
}

impl Config {
    pub fn new() -> Self {
        Self {
            round_time: 90,
            token: None,
            lexicon: Lexicon::Server(0x7671b09a),
            lexicon_lock: false,
        }
    }
}

pub enum Lexicon {
    Upload(Vec<String>),
    Server(u32),
    Git(Vec<String>),
    _None
}

impl Lexicon {
    pub(crate) async fn next_word(&self) -> Option<String> {
        match self {
            Lexicon::Upload(lexicon) => {
                let idx = rand::random::<usize>()%lexicon.len();
                Some(lexicon[idx].clone())
            },
            Lexicon::Server(lexcodes) => {
                // let idx = rand::random::<usize>()%lexcodes.len();
                let query = format!("http://{}/rand-word/{:08x}", LEX_SERVER, lexcodes);
                if let Ok(resp) = reqwest::get(query).await {
                    if let Ok(word) = resp.text().await {
                        return Some(word);
                    }
                } 
                None
            },
            Lexicon::_None => None,
            Lexicon::Git(_) => todo!(),
        }
    }
}