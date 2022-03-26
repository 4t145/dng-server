#[derive(Debug, Clone)]
pub enum Stage {
    Unready,
    Ready,
    Drawing(usize),
    Marking(usize),
    Over
}

pub struct State {
    pub topic: String,
    pub stage: Stage,
    pub player_count: u8,
}

impl State {
    pub fn new() -> Self {
        Self {
            stage: Stage::Unready,
            player_count: 0,
            topic: "当你看到这个，意味着设置关键词的逻辑上出现了bug".to_string(),
        }
    }
}