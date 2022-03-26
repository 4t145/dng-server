pub fn log(msg:String) {
    use chrono::{Local};
    let date = Local::now();
    println!("[{}][INFO]{}",date.format("%Y-%m-%d][%H:%M:%S") , msg)
}