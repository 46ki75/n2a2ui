#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("notionrs error: {0}")]
    NotionRs(#[from] notionrs::error::Error),

    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
