use std::collections::HashMap;
use std::error::Error;
use std::result::Result;

use sesame_mysql::{PConOpts, PConStatement, SesameConn};
use slog::{debug, o};

pub struct MySqlBackend {
    pub handle: SesameConn,
    pub log: slog::Logger,
    pub(super) _schema: String,
    pub(super) prep_stmts: HashMap<String, PConStatement>,
    pub(super) db_user: String,
    pub(super) db_password: String,
    pub(super) db_addr: String,
    pub(super) db_name: String,
}

impl MySqlBackend {
    pub fn new(
        user: &str,
        password: &str,
        addr: &str,
        dbname: &str,
        log: Option<slog::Logger>,
        prime: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let log = match log {None => slog::Logger::root(slog::Discard, o!()), Some(l) => l};

        // Embedded at compile time so the binary does not depend on the working directory.
        let schema = include_str!("schema.sql");

        debug!(
            log,
            "Connecting to MySql DB and initializing schema {}...", dbname
        );
        let mut db = SesameConn::new(
            PConOpts::from_url(&format!("mysql://{}:{}@{}/", user, password, addr)).unwrap(),
        )
        .unwrap();
        assert_eq!(db.ping(), true);

        if prime {
            /*
            db.query_drop(format!("DROP DATABASE IF EXISTS {};", dbname))
                .unwrap();
            db.query_drop(format!("CREATE DATABASE {};", dbname))
                .unwrap();
            db.query_drop(format!("USE {};", dbname)).unwrap();
            */
            // Statements may span several lines: accumulate until a line ends the
            // statement with a ';'.
            let mut cmd = String::from("");
            for line in schema.lines() {
                let line = line.trim();
                if line.starts_with("--") || line.is_empty() {
                    continue;
                }
                cmd += line;
                cmd += " ";
                if line.ends_with(";") {
                    println!("{}", cmd);
                    db.query_drop(cmd).unwrap();
                    cmd = String::from("");
                }
            }
        } else {
            //db.query_drop(format!("USE {};", dbname)).unwrap();
        }

        Ok(MySqlBackend {
            handle: db,
            log: log,
            _schema: schema.to_owned(),
            prep_stmts: HashMap::new(),
            db_user: String::from(user),
            db_password: String::from(password),
            db_addr: String::from(addr),
            db_name: String::from(dbname),
        })
    }

    /// Reconnect if the connection has gone away, and drop the prepared
    /// statement cache with it: prepared statements do not survive a reconnect.
    /// Queries and writes call this before touching the handle.
    pub(super) fn ping_or_reconnect(&mut self) {
        if self.handle.ping() {
            return;
        }

        self.handle = SesameConn::new(
            PConOpts::from_url(&format!(
                "mysql://{}:{}@{}/{}",
                self.db_user, self.db_password, self.db_addr, self.db_name
            ))
            .unwrap(),
        )
        .unwrap();
        self.prep_stmts.clear();
    }
}
