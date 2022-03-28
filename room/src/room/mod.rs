// todo
// 1. 把 <timer> 从 <onshot> 改成 <Notify>
// 2. 拆分stage逻辑

mod config;
mod state;
mod request;
mod error;

use {
    std::collections::HashMap,
    std::time::Duration,

    tokio::sync::mpsc::{Sender, Receiver, channel},
    tokio::sync::{watch, oneshot},
    tokio::time::sleep,
};

use {
    config::Config,
    state::State,
    error::{RoomResult, ErrorKind},
    request::{TimerType},
};

use crate::{
    consts::*,
    room::config::Lexicon,
    types::*,
    player::*,
    observer::{Observer},
};


pub use request::Request as RoomReq;

use protocol::{
    PlayerRequest as PlayerReq, 
    PlayerResponse as PlayerResp, 
    PlayerState, 
    ObserverResponse as ObResp,
    Lexicon as LexiconData
};

pub struct Room {
    name: Option<String>,
    config: Config,
    state: State,

    players: [Option<Player>; ROOM_SIZE],
    observers: HashMap<String, Observer>,

    rm_rx: Receiver<RoomReq>,
    loopback: Sender<RoomReq>,

    token_tx: watch::Sender<Option<u128>>
}

impl Room {
    pub fn new(name:Option<String>) -> Self {
        let (loopback, rm_rx) = channel::<RoomReq>(32);
        let (token_tx, _) = watch::channel(None);

        Self {
            name,
            config: Config::new(),
            state: State::new(),

            observers: HashMap::new(),
            players: [None, None, None, None, None, None, None, None],

            rm_rx,
            loopback,

            token_tx
        }
    }

    pub fn _abort(&mut self) {
        for p in &mut self.players {
            if let Some(player) = p{
                player._abort();
            }
        }
        self.players = Default::default();
        self.state.player_count = 0;
        for (_,ob) in &mut self.observers {
            ob._abort()
        }
        self.observers.clear()
    }

    #[inline]
    pub fn get_tx(&self) -> Sender<RoomReq> {
        self.loopback.clone()
    }
    
    async fn login_player(&mut self, ws_stream: WsStream) -> RoomResult<()> {
        for idx in 0..ROOM_SIZE {
            if self.players[idx].is_none() {
                let player = Player::new(idx, ws_stream, self.get_tx());
                self.players[idx] = Some(player);
                self.state.player_count += 1;
                self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                self.feed(self.get_room_states()).await;
                return Ok(());
                // return Ok(idx);
            }
        };
        return Err(ErrorKind::RoomFullfilled);
    }

    async fn login_observer(&mut self, ws_stream: WsStream, name: String, addr: String) {
        let ob = Observer::new(name, addr, ws_stream, self.loopback.clone());
        let key = ob.localname.clone();
        ob.send(self.get_room_states()).await;
        self.observers.insert(key, ob);
    }

    async fn logout(&mut self, idx: usize) {
        if self.players[idx].take().is_some() {
            self.state.player_count -= 1;
        }
        self.feed(self.get_room_states()).await;
        self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
    }


    fn logout_observer(&mut self, name:&String) {
        self.observers.remove(name);
    }
    pub fn subscribe_token(&self) -> watch::Receiver<Option<u128>> {
        self.token_tx.subscribe()
    }
    

    pub async fn run(&mut self) {
        use state::Stage;
        let mut timer = None;
        let mut timepoint = 0;
        let github_re = regex::Regex::new(r"^https?://github.com/(.*)/(.*)/blob/(.*\.json)$").unwrap();
        loop{ 
            // if signal. {
            //     self._abort();
            //     return;
            // }

            match self.state.stage {
                Stage::Unready => {
                    if let Some(req) = self.rm_rx.recv().await {
                        match req {
                            RoomReq::PlayerReq(idx, req) => {
                                match req {
                                    PlayerReq::SetName { name}  => {
                                        self.players[idx].as_mut().unwrap().set_name(name);
                                        self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                                    },
                                    PlayerReq::Chat {msg} => {
                                        let sender_name = self.players[idx].as_ref().and_then(|p|Some(p.get_name())).unwrap_or_default();
                                        self.broadcast_except(PlayerResp::Chat {sender: sender_name, msg: msg}, idx).await;
                                    },
                                    PlayerReq::ImReady  => {
                                        self.players[idx].as_mut().unwrap().ready();
                                        self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                                        if self.every_one_ready() {
                                            self.state.stage = Stage::Ready;
                                            self.broadcast(PlayerResp::GameStart).await;
                                            self.feed(self.get_room_states()).await;
                                        }
                                    },
                                    PlayerReq::ImUnready => {
                                        self.players[idx].as_mut().unwrap().unready();
                                        self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                                    },
                                    PlayerReq::Lexicon (lexicon)=> {
                                        self.config.lexicon = Lexicon::Upload(lexicon);
                                        let msg = format!("{} 上传了词库", self.get_name(idx));
                                        self.broadcast(PlayerResp::Notice{msg}).await;
                                    }
                                    PlayerReq::LexiconService (lexcodes) => {
                                        let msg = format!("{} 加载了服务器词库, 正在获取词库信息", self.get_name(idx));
                                        self.broadcast(PlayerResp::Notice{msg}).await;
                                        match reqwest::get(format!("http://{}/name/{}", LEX_SERVER, lexcodes)).await {
                                            Ok(res) => {
                                                match res.error_for_status() {
                                                    Ok(ok_res) => {
                                                        let name = ok_res.text().await.unwrap_or("<无法解析的名称>".to_string());
                                                        let msg = format!("词库可用！词库名称：{}", name);
                                                        self.broadcast(PlayerResp::Notice { msg }).await;
                                                        self.config.lexicon = Lexicon::Server(lexcodes);
                                                    }
                                                    Err(err) => {
                                                        let msg = format!("词库响应 {}， 请检查你的词库码是否正确，不过也可能是词库挂了", err.status().unwrap_or_default());
                                                        self.broadcast(PlayerResp::Warn { msg }).await;
                                                    }
                                                }
                                            },
                                            Err(err) => {
                                                let msg = format!("怎么会是呢，词库没反应啊！错误消息：{}", err);
                                                self.broadcast(PlayerResp::Warn { msg }).await;
                                            },
                                        }
                                    },
                                    PlayerReq::LexiconGit (path) => {
                                        // for example: https://github.com/4t145/repo/blob/master/dirname/filename.json
                                        let msg = format!("{} 加载了Github词库, 正在获取词库信息", self.get_name(idx));
                                        self.broadcast(PlayerResp::Notice{msg}).await;
                                        if let Some(caps) = github_re.captures(path.as_str()) {
                                            if caps.len() == 4 {
                                                let user = caps.get(1).unwrap().as_str();
                                                let repo = caps.get(2).unwrap().as_str();
                                                let file = caps.get(3).unwrap().as_str();
                                                let resource = format!("https://raw.githubusercontent.com/{}/{}/{}", user, repo, file);
                                                let msg = format!("尝试下载资源 {}", resource);
                                                self.broadcast(PlayerResp::Notice{msg}).await;
                                                match reqwest::get(resource).await {
                                                    Ok(res) => {
                                                        match res.error_for_status() {
                                                            Ok(ok_res) => {
                                                                self.broadcast(PlayerResp::Notice{msg: "尝试解码".to_string()}).await;
                                                                if let Ok(bytes) = ok_res.bytes().await {
                                                                    if let Ok(lexicon_data) = serde_json::from_slice::<LexiconData>(&bytes) {
                                                                        let msg = format!("已下载词库！名称: {}， 容量{}", lexicon_data.name, lexicon_data.lexicon.len());
                                                                        self.broadcast(PlayerResp::Notice{msg}).await;
                                                                        self.config.lexicon = Lexicon::Git(lexicon_data)
                                                                    } else {
                                                                        let msg = format!("不合法的词库文件，请检查词库是否具有各自段");
                                                                        self.broadcast(PlayerResp::Warn{msg}).await;
                                                                    }
                                                                } else {
                                                                    let msg = format!("无法解码，请检查词库格式/地址");
                                                                    self.broadcast(PlayerResp::Warn{msg}).await;
                                                                }
                                                            }
                                                            Err(err) => {
                                                                let msg = format!("网络错误， 状态码：{}", err.status().unwrap_or_default());
                                                                self.broadcast(PlayerResp::Warn { msg }).await;
                                                            }
                                                        }
                                                    },
                                                    Err(err) => {
                                                        let msg = format!("网络错误：{}", err);
                                                        self.broadcast(PlayerResp::Warn { msg }).await;
                                                    },
                                                }
                                            } else {
                                                let msg = format!("地址只对了一半捏");
                                                self.broadcast(PlayerResp::Warn { msg }).await;
                                            }
                                        } else {
                                            let msg = format!("地址似乎不太对捏");
                                            self.broadcast(PlayerResp::Warn { msg }).await;
                                        }

                                    },
                                    _ => {}
                                }
                            }
                            RoomReq::Timer(_) => {},
                            RoomReq::CountDown(_) => {},
                            RoomReq::ObserverReq(_, _) => {},
                            RoomReq::PlayerLogin{ws_stream} => {
                                self.login_player(ws_stream).await.unwrap_or_default();
                            },
                            RoomReq::ObserverLogin { addr, name, ws_stream } => {
                                self.login_observer(ws_stream, name, addr).await;
                            },
                            RoomReq::PlayerLogout(idx) => {self.logout(idx).await;},
                            RoomReq::ObserverLogout(name) => {self.logout_observer(&name)},
                        }
                    }
                },
                Stage::Ready => {
                    self.state.stage = if let Some(first) = self.get_next_drawer(None) {
                        self.broadcast( PlayerResp::TurnStart(first as u8)).await;
                        self.state.topic = self.config.lexicon.next_word().await.unwrap_or_default();
                        self.send(first, PlayerResp::Topic {
                            topic_word: self.state.topic.clone()
                        }).await;
                        timer = Some(self.set_timer(self.config.round_time, TimerType::Draw));
                        timepoint = 0;
                        Stage::Drawing(first)
                    } else {
                        Stage::Unready
                    };
                    // self.feed(self.get_room_states()).await;
                },
                Stage::Drawing(drawer_idx) => {
                    let mut timesup = false;
                    if let Some(req) = self.rm_rx.recv().await {
                        match req {
                            RoomReq::PlayerReq(idx, req) => {
                                match req {
                                    PlayerReq::Chat {msg} => {
                                        if idx != drawer_idx {
                                            let sender_name = self.players[idx].as_ref().unwrap().name.clone();
                                            // logic when sb. get the answer
                                            if self.check_answer(&msg) {
                                                if let Some(ref mut player) = self.players[idx] {
                                                    player.set_right(drawer_idx);
                                                    player.timepoint += timepoint;
                                                    self.send(idx, PlayerResp::Notice {msg: "正确答案！".to_string()}).await;
                                                    self.broadcast_except(
                                                        PlayerResp::Notice {msg: format!("{} 已经猜到了正确答案!", sender_name)}, 
                                                    idx).await;
                                                }
                                                self.players[idx].as_mut()
                                                .and_then(|p|Some(p.add_draw_point()))
                                                .unwrap_or_default();
                                            } else {
                                                self.broadcast_except(PlayerResp::Chat {sender: sender_name, msg: msg}, idx).await;
                                            }
                                        }
                                    },
                                    protocol::PlayerRequest::Chunk { bin } => {
                                        if drawer_idx == idx {
                                            let req = PlayerResp::Chunk {bin};
                                            self.broadcast_except(req, idx).await;
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            RoomReq::Timer(TimerType::Draw) => {timesup = true;},
                            RoomReq::CountDown(rest) => {
                                self.broadcast( PlayerResp::CountDown(rest as u8)).await;
                                timepoint += 1;
                            },
                            RoomReq::ObserverLogin { addr, name, ws_stream } => {
                                self.login_observer(ws_stream, name, addr).await;
                            },
                            RoomReq::PlayerLogout(idx) => {self.logout(idx).await;},
                            RoomReq::ObserverLogout(name) => {self.logout_observer(&name)},
                            _ => {}
                        }
                    }
                    if self.every_one_right(drawer_idx) || timesup ||self.state.player_count==0{
                        self.broadcast(PlayerResp::TurnEnd).await;
                        self.broadcast_except(PlayerResp::Poll, drawer_idx).await;
                        let _ = timer.unwrap().send(());
                        timer = Some(self.set_timer(5, TimerType::Mark));
                        timepoint = 0;
                        self.state.stage = Stage::Marking(drawer_idx);
                        self.broadcast(PlayerResp::MarkStart).await;
                    }
                },
                Stage::Marking(drawer_idx) => {
                    if let Some(req) = self.rm_rx.recv().await {
                        match req {
                            RoomReq::PlayerReq(idx, req) => {
                                match req {
                                    PlayerReq::Chat {msg} => {
                                        let sender_name = self.players[idx].as_ref().unwrap().name.clone();
                                        self.broadcast_except(PlayerResp::Chat {sender: sender_name, msg: msg}, idx).await;
                                    },
                                    PlayerReq::Mark{score} => {
                                        // <temp!
                                        self.broadcast(PlayerResp::Notice {
                                            msg: format!("{} 对此的评价是 {}", 
                                                self.players[idx].as_ref().unwrap().get_name(), 
                                                match score {
                                                    -1 => "差评如潮",
                                                    0 => "褒贬不一",
                                                    1 => "好评如潮",
                                                    _ => "好评如潮",
                                                }
                                            )
                                        }, ).await;
                                        // !temp>
                                        if let Some(ref mut player) = self.players[drawer_idx] { player.mark(score); };
                                        self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                                    },
                                    _ => {}
                                }
                            },
                            RoomReq::Timer(TimerType::Mark) => {
                                self.broadcast( PlayerResp::MarkEnd).await;
                                self.state.stage = if let Some(next) = self.get_next_drawer(Some(drawer_idx)) {
                                    self.broadcast( PlayerResp::TurnStart(next as u8)).await;
                                    self.state.topic = self.config.lexicon.next_word().await.unwrap_or_default();
                                    self.send(next, PlayerResp::Topic {
                                        topic_word: self.state.topic.clone()
                                    }).await;
                                    timer = Some(self.set_timer(self.config.round_time, TimerType::Draw));
                                    Stage::Drawing(next)
                                } else {
                                    Stage::Over
                                };
                            },
                            RoomReq::CountDown(rest) => {
                                self.broadcast( PlayerResp::CountDown(rest as u8)).await;
                            },
                            RoomReq::PlayerLogout(idx) => {
                                self.logout(idx).await;
                            },
                            RoomReq::ObserverLogout(name) => {self.logout_observer(&name)},
                            _ => {}
                        }
                    }
                }
                Stage::Over => {
                    self.broadcast(PlayerResp::GameEnd).await;
                    self.reset_all();
                    self.broadcast(PlayerResp::PlayerStates(self.player_states())).await;
                    self.state.stage = Stage::Unready;
                    self.feed(self.get_room_states()).await;
                },

                // unreachable by now
                Stage::_Recover => {
                    self._abort();
                    self.reset_all();
                    self.state.stage = Stage::Unready;
                    unreachable!();
                }
            }
        }
    }


    #[inline]
    pub async fn send(&mut self, idx:usize, resp: PlayerResp) {
        if let Some(ref player ) = self.players[idx] {
            player.send(resp).await
        }
    }

    async fn broadcast(&self, resp: PlayerResp) {
        for player in self.players.iter() {
            if let Some(p) = player {
                p.send(resp.clone()).await;
            }
        }
    }

    async fn broadcast_except(&self, resp: PlayerResp, except: usize) {
        for idx in 0..ROOM_SIZE {
            if self.players[idx].is_some() && idx != except {
                let player = self.players[idx].as_ref().unwrap();
                player.send(resp.clone()).await;
            }
        }
    }

    async fn feed(&self, resp: ObResp) {
        for (_,  ob) in &self.observers {
            ob.send(resp.clone()).await;
        }
    }

    fn set_timer(&self, secs:usize, timer_type: TimerType) -> oneshot::Sender<()> {
        let (tx, mut rx) = oneshot::channel::<()>();
        let loopback_tx = self.loopback.clone();
        tokio::spawn(async move {
            for passed in 0..secs {
                sleep(Duration::from_secs(1)).await;
                if rx.try_recv().is_ok() {
                    return;
                }
                loopback_tx.send(RoomReq::CountDown(secs-passed)).await.unwrap();
            }
            loopback_tx.send(RoomReq::Timer(timer_type)).await.unwrap();
        });
        return tx;
    }
    
    fn every_one_ready(&self) -> bool {
        if self.state.player_count < 2 {
            return false;
        };
        let mut is_all_ready = true;
        for idx in 0..ROOM_SIZE {
            if let Some(ref player) = self.players[idx] {
                is_all_ready = is_all_ready&&player.ready;
            }
        }
        is_all_ready
    }

    fn every_one_right(&self, drawer_idx: usize) -> bool {
        let mut is_every_one_right = true;
        for idx in 0..ROOM_SIZE {
            if idx == drawer_idx {
                continue;
            }
            if let Some(ref player) = self.players[idx] {
                is_every_one_right = is_every_one_right&&player.right[drawer_idx];
            }
        }
        is_every_one_right
    }

    #[inline]
    fn get_name(&self, idx:usize) -> String {
        self.players[idx].as_ref().unwrap().name.clone()
    }

    fn player_states(&self) -> Vec<PlayerState> {
        let mut res = Vec::with_capacity(ROOM_SIZE);
        for idx in 0..ROOM_SIZE {
            if let Some(ref player) = self.players[idx] {
                res.push(player.get_state(idx));
            }
        }
        res
    }
    
    fn get_room_states(&self) -> ObResp {
        let stage = match self.state.stage {
            state::Stage::Unready => "unready",
            _ => "gaming"
        }.to_string();

        let playercount = self.state.player_count as u8;
        let user_lexicon = match self.config.lexicon {
            config::Lexicon::Upload(_) => true,
            config::Lexicon::Git(_) => true,
            config::Lexicon::Server(_) => false,
            config::Lexicon::_None => false,
        };
        let lexicon = match self.config.lexicon {
            config::Lexicon::Server(ref codes) => codes.clone(),
            _ => 0
        };
        let name = self.name.clone();
        ObResp::RoomState(protocol::RoomState{
            stage, playercount, user_lexicon, lexicon, name
        })
    }


    fn get_next_drawer(&self, current: Option<usize>) -> Option<usize> {
        let mut start = 0;
        if let Some(current_drawer) = current {
            start = current_drawer+1;
        }
        for idx in start..ROOM_SIZE {
            if let Some(ref player) = self.players[idx] {
                if player.ready {
                    return Some(idx);
                }
            }
        }
        return None;
    }

    #[inline]
    fn check_answer(&self, answer:&String) -> bool {
        answer.contains(&self.state.topic)
    }

    fn reset_all(&mut self) {
        for idx in 0..ROOM_SIZE {
            if let Some(ref mut player) = self.players[idx] {
                player.reset();
            }
        }
    }
}