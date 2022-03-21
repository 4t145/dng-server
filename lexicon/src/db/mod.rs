

use std::num::ParseIntError;

use sqlx::{SqlitePool, Row};
use async_trait::async_trait;

#[derive(Debug)]
pub enum ErrorKind {
    Sqlx(sqlx::Error),
    Parse(ParseIntError),
}

fn sqlx_err(e: sqlx::Error) -> ErrorKind {
    ErrorKind::Sqlx(e)
}

fn parse_err(e: ParseIntError) -> ErrorKind {
    ErrorKind::Parse(e)
}

#[async_trait]
pub trait Query {
    type Reply;
    async fn query(&self, pool: &SqlitePool) -> Result<Self::Reply, ErrorKind>;
}
pub struct RandWord(pub String);

#[async_trait]
impl Query for RandWord {
    type Reply = String;
    async fn query(&self, pool: &SqlitePool) -> Result<Self::Reply, ErrorKind> {
        dbg!(self.0.clone());
        let query = format!("select word from '{}' order by random() limit 1;", self.0.clone());
        let row = sqlx::query(query.as_str()).fetch_one(pool)
        .await.map_err(sqlx_err)?;
        dbg!("fetch row");
        let word:&str = row.try_get("word").map_err(sqlx_err)?;
        dbg!(word);

        return Ok(word.to_string());
    }
}

pub struct Name(pub String);

#[async_trait]
impl Query for Name {
    type Reply = String;
    async fn query(&self, pool: &SqlitePool) -> Result<Self::Reply, ErrorKind> {
        dbg!(self.0.clone());
        let dig = u32::from_str_radix(self.0.as_str(), 16).map_err(parse_err)?;
        let query = format!("select name from lexindex where lexcode='{:08x}';", dig);
        let row = sqlx::query(query.as_str()).fetch_one(pool)
        .await.map_err(sqlx_err)?;
        dbg!("fetch row");
        let name:&str = row.try_get("name").map_err(sqlx_err)?;
        dbg!(name);

        return Ok(name.to_string());
    }
}















// pub struct SelectLexicon {
//     name: Option<String>,
//     author: Option<String>,
//     lang: Option<String>,
//     tags: Vec<String>
// }

// pub struct LexiconItem {
//     lexicode: String,
//     name: String,
//     lang: String,
//     version: Option<String>,
//     author: Option<String>,
//     tags: String,
//     size: u32,
// }

// const SELECT_LEXICON_QUERY:&str = 
// r#"select lexcode, name, author, lang, tags from lexindex 
//     where email = ? OR name = ?;
// "#;
// #[async_trait]
// impl<'q> Query for SelectLexicon<'q> {
//     type Reply = String;
//     async fn query(&self, conn: &mut SqliteConnection) -> Result<Self::Reply, sqlx::Error> {

//         return Ok(word.to_string());
//     }
// }