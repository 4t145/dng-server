pub enum ErrorKind {
    RoomFullfilled,
    _Unknown,
}

pub type RoomResult<T> = Result<T, ErrorKind>;

