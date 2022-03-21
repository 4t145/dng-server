
pub struct Config {
    pub token: Option<u128>,
    pub lexicon: Lexicon,
    pub lexicon_lock: bool,
}

impl Config {
    pub fn new() -> Self {
        Self {
            token: None,
            lexicon: Lexicon::None,
            lexicon_lock: false,
        }
    }
}

pub enum Lexicon {
    Upload(Vec<String>),
    Server(u32),
    None
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
                let query = format!("http://127.0.0.1:3030/rand-word/{:08x}",lexcodes);
println!("{}", query);
                if let Ok(resp) = reqwest::get(query).await {
dbg!(&resp);
                    if let Ok(word) = resp.text().await {
dbg!(&word);
                        return Some(word);
                    }
                } 
                None
            },
            Lexicon::None => None,
        }
    }
}