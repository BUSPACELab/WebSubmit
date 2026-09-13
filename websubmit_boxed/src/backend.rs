use crate::policies::ContextData;
use sesame::context::Context;
use sesame_mysql::{SesameConn, PConOpts, PConParams, PConRow, PConStatement};
use slog::{debug, o, warn};
use std::collections::HashMap;
use std::error::Error;
use std::result::Result;

pub struct MySqlBackend {
    pub handle: SesameConn,
    pub log: slog::Logger,
    _schema: String,
    prep_stmts: HashMap<String, PConStatement>,
    db_user: String,
    db_password: String,
    db_name: String,
}

impl MySqlBackend {
    pub fn new(
        user: &str,
        password: &str,
        dbname: &str,
        log: Option<slog::Logger>,
        prime: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let log = match log {
            None => slog::Logger::root(slog::Discard, o!()),
            Some(l) => l,
        };

        // Embedded at compile time so the binary does not depend on the working directory.
        let schema = include_str!("schema.sql");

        debug!(
            log,
            "Connecting to MySql DB and initializing schema {}...", dbname
        );
        // let password = "";
        // println!("password is `{}`", password);
        let mut db = SesameConn::new(
            // this is the user and password from the config.toml file
            PConOpts::from_url(&format!("mysql://{}:{}@127.0.0.1/", user, password)).unwrap(),
        )
        .unwrap();
        assert_eq!(db.ping(), true);

        if prime {
            db.query_drop(format!("DROP DATABASE IF EXISTS {};", dbname))
                .unwrap();
            db.query_drop(format!("CREATE DATABASE {};", dbname))
                .unwrap();
            db.query_drop(format!("USE {};", dbname)).unwrap();
            for line in schema.lines() {
                if line.starts_with("--") || line.is_empty() {
                    continue;
                }
                db.query_drop(line).unwrap();
            }
        } else {
            db.query_drop(format!("USE {};", dbname)).unwrap();
        }

        Ok(MySqlBackend {
            handle: db,
            log: log,
            _schema: schema.to_owned(),
            prep_stmts: HashMap::new(),
            db_user: String::from(user),
            db_password: String::from(password),
            db_name: String::from(dbname),
        })
    }

    fn reconnect(&mut self) {
        self.handle = SesameConn::new(
            PConOpts::from_url(&format!(
                "mysql://{}:{}@127.0.0.1/{}",
                self.db_user, self.db_password, self.db_name
            ))
            .unwrap(),
        )
        .unwrap();
    }

    pub fn prep_exec<P: Into<PConParams>>(
        &mut self,
        sql: &str,
        params: P,
        context: Context<ContextData>,
    ) -> Vec<PConRow> {
        if !self.handle.ping() {
            self.reconnect();
            self.prep_stmts.clear();
        }
        if !self.prep_stmts.contains_key(sql) {
            let stmt = self
                .handle
                .prep(sql)
                .expect(&format!("failed to prepare statement \'{}\'", sql));
            self.prep_stmts.insert(sql.to_owned(), stmt);
        }

        // PConParams is not Clone in sesame, so the query cannot be replayed with the
        // same params. Reconnect up front instead of retrying after a failure.
        let params: PConParams = params.into();
        match self
            .handle
            .exec_iter(self.prep_stmts[sql].clone(), params, context.clone())
        {
            Err(e) => {
                warn!(self.log, "query \'{}\' failed ({})", sql, e);
                panic!("query \'{}\' failed ({})", sql, e);
            }
            Ok(res) => {
                let mut rows = vec![];
                for row in res {
                    rows.push(row.unwrap());
                }
                //debug!(self.log, "executed query {}, got {} rows", sql, rows.len());
                return rows;
            }
        }
    }

    fn do_insert<P: Into<PConParams>>(
        &mut self,
        table: &str,
        vals: P,
        replace: bool,
        context: Context<ContextData>,
    ) {
        if !self.handle.ping() {
            self.reconnect();
            self.prep_stmts.clear();
        }
        let vals: PConParams = vals.into();
        let mut param_count = 0;
        if let PConParams::Positional(vec) = &vals {
            param_count = vec.len();
        }

        let op = if replace { "REPLACE" } else { "INSERT" };
        let q = format!(
            "{} INTO {} VALUES ({})",
            op,
            table,
            (0..param_count)
                .map(|_| "?")
                .collect::<Vec<&str>>()
                .join(",")
        );
        // See prep_exec: params cannot be cloned, so this cannot be replayed.
        if let Err(e) = self.handle.exec_drop(q.clone(), vals, context.clone()) {
            warn!(
                self.log,
                "failed to insert into {}, query {} ({})", table, q, e
            );
            panic!("failed to insert into {} ({})", table, e);
        }
    }

    pub fn insert<P: Into<PConParams>>(
        &mut self,
        table: &str,
        vals: P,
        context: Context<ContextData>,
    ) {
        self.do_insert(table, vals, false, context);
    }

    pub fn replace<P: Into<PConParams>>(
        &mut self,
        table: &str,
        vals: P,
        context: Context<ContextData>,
    ) {
        self.do_insert(table, vals, true, context);
    }
}
