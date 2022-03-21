// use crate::logger::log;

// #[derive(Debug)]
// pub struct Error {
//     code: u8,
//     msg: String
// }

// impl Error {
//     pub fn undef_err(msg: String) -> Self {
//         Self {
//             code: 0x00,
//             msg: msg.to_owned()
//         }
//     }

//     pub fn room_unfilled() -> Error { 
//         Error {
//             code: 0x10,
//             msg: "try to loging while room is filled".to_owned()
//         }
//     }

//     pub fn deserde_fail() -> Error {
//         Error { 
//             code: 0x11, 
//             msg: "cannot deserde message".to_owned()
//         }
//     }

//     pub fn should_log() -> Error {
//         Error { 
//             code: 0x12, 
//             msg: "cannot deserde message".to_owned()
//         }
//     }
// }


// impl Drop for Error {
//     fn drop(&mut self) {
//         log(format!("unhandled error: [0x{:02x}]: {}", self.code, self.msg));
//     }
// }