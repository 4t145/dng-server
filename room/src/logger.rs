pub fn log(msg:String) {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    println!("[{:?}][INFO]{}",now , msg)
}