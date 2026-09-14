use sesame::context::Context;
use sesame_mysql::PConParams;
use slog::warn;

use crate::db::MySqlBackend;
use crate::policies::ContextData;

impl MySqlBackend {
    /// Run a statement that returns no rows (UPDATE, DELETE).
    ///
    /// Reads go through `query_<table>` instead; this is the write-side
    /// counterpart for statements that change rows in place.
    pub fn exec<P: Into<PConParams>>(
        &mut self,
        sql: &str,
        params: P,
        context: Context<ContextData>,
    ) {
        self.ping_or_reconnect();

        // See do_insert: params cannot be cloned, so this cannot be replayed.
        if let Err(e) = self.handle.exec_drop(sql, params.into(), context) {
            warn!(self.log, "statement \'{}\' failed ({})", sql, e);
            panic!("statement \'{}\' failed ({})", sql, e);
        }
    }

    /// INSERT or REPLACE a full row into `table`.
    ///
    /// `table` may carry an explicit column list, e.g. "presenters(lecture_id, email)".
    fn do_insert<P: Into<PConParams>>(
        &mut self,
        table: &str,
        vals: P,
        replace: bool,
        context: Context<ContextData>,
    ) {
        self.ping_or_reconnect();

        let vals: PConParams = vals.into();
        let mut param_count = 0;
        if let PConParams::Positional(vec) = &vals {
            param_count = vec.len();
        }

        let op = if replace { "REPLACE" } else {"INSERT"};
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
        if let Err(e) = self.handle.exec_drop(q.clone(), vals, context) {
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
