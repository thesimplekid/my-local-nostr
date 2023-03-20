use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("DB Error")]
    DBError(redb::Error),
    #[error("Not in db")]
    NotFound,
    #[error("Serde error")]
    SerdeError(serde_json::Error),
    #[error("Join error")]
    JoinError(tokio::task::JoinError),
    #[error("Invoice Error")]
    InvoiceError,
}

impl From<redb::Error> for Error {
    fn from(err: redb::Error) -> Self {
        Self::DBError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeError(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::JoinError(err)
    }
}
