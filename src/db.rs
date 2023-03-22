use redb::{Database, ReadableTable, TableDefinition};
use tracing::debug;

use crate::{error::Error, Users};
// key is hex pubkey value is name
const ACCOUNTTABLE: TableDefinition<&str, u8> = TableDefinition::new("account");
const EVENTTABLE: TableDefinition<&str, u8> = TableDefinition::new("event");

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum Status {
    Deny,
    Allow,
}

impl Status {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Status::Deny,
            1 => Status::Allow,
            // This should never happen
            _ => Status::Deny,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    pub pubkey: String,
    pub status: Status,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub status: Status,
}

impl Account {
    pub fn is_admitted(&self) -> bool {
        if self.status.eq(&Status::Allow) {
            return true;
        }
        false
    }
}

impl Event {
    pub fn is_admitted(&self) -> bool {
        if self.status.eq(&Status::Allow) {
            return true;
        }
        false
    }
}

pub struct Db {
    db: Database,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

impl Db {
    pub fn new() -> Self {
        debug!("Creating DB");
        let db = Database::create("my_db.redb").unwrap();
        //  db.set_write_strategy(WriteStrategy::TwoPhase).unwrap();
        let write_txn = db.begin_write().unwrap();
        {
            // Opens the table to create it
            let _ = write_txn.open_table(ACCOUNTTABLE).unwrap();
            let _ = write_txn.open_table(EVENTTABLE).unwrap();
        }
        write_txn.commit().unwrap();

        Self { db }
    }

    pub fn write_account(&self, account: &Account) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(ACCOUNTTABLE)?;
            table.insert(account.pubkey.as_str(), account.status as u8)?;
        }
        write_txn.commit().unwrap();
        Ok(())
    }

    pub fn read_account(&self, pubkey: &str) -> Result<Option<Account>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACCOUNTTABLE)?;
        if let Some(account_info) = table.get(pubkey)? {
            let account = Account {
                pubkey: pubkey.to_string(),
                status: Status::from_u8(account_info.value()),
            };
            return Ok(Some(account));
        }
        Ok(None)
    }

    pub fn read_all_accounts(&self) -> Result<(), Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACCOUNTTABLE)?;

        for a in table.iter()? {
            debug!("{:?}, {}", a.0.value(), a.1.value());
        }
        Ok(())
    }

    pub fn read_accounts(&self) -> Result<Users, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACCOUNTTABLE)?;

        let users: Vec<(String, u8)> = table
            .iter()?
            .map(|(k, s)| (k.value().to_string(), s.value()))
            .collect();

        let (allow, deny): (Vec<String>, Vec<String>) =
            users
                .iter()
                .map(|(s, i)| (s, *i))
                .fold((Vec::new(), Vec::new()), |mut acc, (s, i)| {
                    match i {
                        1 => acc.0.push(s.to_owned()),
                        0 => acc.1.push(s.to_owned()),
                        _ => {}
                    }
                    acc
                });

        Ok(Users {
            allow: Some(allow),
            deny: Some(deny),
        })
    }

    pub fn write_event(&self, event: &Event) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(EVENTTABLE)?;
            table.insert(event.id.as_str(), event.status as u8)?;
        }
        write_txn.commit().unwrap();
        Ok(())
    }
    pub fn write_events(&self, events: &[Event]) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(EVENTTABLE)?;
            for event in events {
                table.insert(event.id.as_str(), event.status as u8)?;
            }
        }
        write_txn.commit().unwrap();
        Ok(())
    }

    pub fn read_event(&self, event_id: &str) -> Result<Option<Event>, Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTTABLE)?;
        if let Some(event_info) = table.get(&event_id)? {
            let event = Event {
                id: event_id.to_owned(),
                status: Status::from_u8(event_info.value()),
            };
            return Ok(Some(event));
        }
        Ok(None)
    }

    pub fn read_all_events(&self) -> Result<(), Error> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTTABLE)?;

        for a in table.iter()? {
            debug!("{:?}, {}", a.0.value(), a.1.value());
        }
        Ok(())
    }

    pub fn clear_tables(&self) -> Result<(), Error> {
        let write_txn = self.db.begin_write()?;

        {
            let mut table = write_txn.open_table(ACCOUNTTABLE)?;
            while table.len()? > 0 {
                let _ = table.pop_first();
            }
        }
        write_txn.commit().unwrap();

        Ok(())
    }
}
