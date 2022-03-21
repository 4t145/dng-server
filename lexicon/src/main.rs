use clap::Parser;
use db::Query;
use sqlx::{SqlitePool};
use warp::{Filter, reject};
mod db;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    db: String,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();
    let pool = SqlitePool::connect(&args.db).await.unwrap();

    let pool_c = pool.clone();
    let rand_word_service = 
    warp::path!("rand-word" / String)
    .and(warp::any().map(move || pool_c.clone()))
    .and_then(rand_word_query);

    let pool_c = pool.clone();
    let name_query_service = 
    warp::path!("name" / String)
    .and(warp::any().map(move || pool_c.clone()))
    .and_then(name_query);


    warp::serve(rand_word_service.or(name_query_service))
        .run(([0, 0, 0, 0], 3030))
        .await;
}


async fn rand_word_query(lexcode:String, pool:SqlitePool) -> Result<String, reject::Rejection> {
    db::RandWord(lexcode).query(&pool).await.map_err(|err| {dbg!(err);reject()})
}


async fn name_query(lexcode:String, pool:SqlitePool) -> Result<String, reject::Rejection> {
    db::Name(lexcode).query(&pool).await.map_err(|err| {dbg!(err);reject()})
}
