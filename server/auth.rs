use std::error;
use std::fmt;
use std::hash::{SipHasher, Hash, Hasher};
use std::path::Path;
use std::result;
use rand;

use rusqlite::{SqliteConnection, SqliteError};
use rusqlite::types::ToSql;
use rusqlite_ffi::SQLITE_CONSTRAINT;

use util::StrError;


pub struct Auth {
    conn: SqliteConnection,
}

impl Auth {
    pub fn new<P: AsRef<Path>>(db_path: &P) -> Result<Auth> {
        let conn = try!(SqliteConnection::open(db_path));
        try!(conn.execute("CREATE TABLE IF NOT EXISTS auth (
                           name      TEXT NOT NULL UNIQUE,
                           secret    TEXT NOT NULL
                           )", &[]));
        Ok(Auth {
            conn: conn,
        })
    }

    pub fn register(&mut self, name: &str, secret: &Secret) -> Result<bool> {
        let hash = hash_secret(secret);
        let result = self.conn.execute("INSERT INTO auth (name, secret)
                                        VALUES ($1, $2)",
                                       &[&name as &ToSql,
                                         &&*hash as &ToSql]);
        match result {
            Ok(_) => Ok(true),
            // Constraint violation means the username is already registered.
            Err(ref e) if e.code == SQLITE_CONSTRAINT => Ok(false),
            Err(e) => Err(Error::Sqlite(e)),
        }
    }

    pub fn login(&mut self, name: &str, secret: &Secret) -> Result<bool> {
        let mut stmt = try!(self.conn.prepare("SELECT secret FROM auth WHERE name = $1"));

        for row in try!(stmt.query(&[&name as &ToSql])) {
            let row = try!(row);
            let hash: String = row.get(0);
            return match check_secret(secret, &*hash) {
                SecretMatch::Yes => Ok(true),
                SecretMatch::No => Ok(false),
                SecretMatch::YesNeedsRehash => {
                    let new_hash = hash_secret(secret);
                    try!(self.conn.execute("UPDATE auth SET secret = $2 WHERE name = $1",
                                           &[&name as &ToSql,
                                             &&*new_hash as &ToSql]));
                    Ok(true)
                },
            };
        }
        Ok(false)
    }
}



pub type Secret = [u32; 4];

fn hash_secret(s: &Secret) -> String {
    // TODO: use a better hash

    let salt0 = rand::random();
    let salt1 = rand::random();

    let mut sip = SipHasher::new_with_keys(salt0, salt1);
    for x in s.iter() {
        x.hash(&mut sip);
    }
    let hash = sip.finish();

    return format!("0;{};{};{}", salt0, salt1, hash);
}

enum SecretMatch {
    Yes,
    No,
    YesNeedsRehash,
}

fn check_secret(s: &Secret, hash: &str) -> SecretMatch {
    // TODO: use a better hash

    let idx = hash.find(';').unwrap();
    let version: u32 = hash[..idx].parse().unwrap();

    if version == 0 {
        let mut iter = hash[(idx + 1)..].split(';');
        let salt0 = iter.next().unwrap().parse().unwrap();
        let salt1 = iter.next().unwrap().parse().unwrap();
        let expect_hash = iter.next().unwrap().parse().unwrap();

        let mut sip = SipHasher::new_with_keys(salt0, salt1);
        for x in s.iter() {
            x.hash(&mut sip);
        }
        let hash = sip.finish();

        if hash == expect_hash {
            SecretMatch::Yes
        } else {
            SecretMatch::No
        }
    } else {
        SecretMatch::No
    }
}


#[derive(Debug)]
pub enum Error {
    Str(StrError),
    Sqlite(SqliteError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Str(ref e) => e.fmt(f),
            Error::Sqlite(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Str(ref e) => e.description(),
            Error::Sqlite(ref e) => &*e.message,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Str(ref e) => Some(e as &error::Error),
            // SqliteError doesn't implement Error.
            Error::Sqlite(_) => None,
        }
    }
}

impl From<StrError> for Error {
    fn from(e: StrError) -> Error {
        Error::Str(e)
    }
}

impl From<SqliteError> for Error {
    fn from(e: SqliteError) -> Error {
        Error::Sqlite(e)
    }
}

pub type Result<T> = result::Result<T, Error>;


