use crate::db::MySqlBackend;
use crate::models::{AnswerModel, PresenterModel, UserModel};

impl MySqlBackend {
    /// Every row across every table that belongs to `email`, via K9db's
    /// `GDPR GET` statement.
    ///
    /// K9db rejects `GDPR GET` as a prepared statement outright (even
    /// `PREPARE ... FROM 'GDPR GET users ?'` errors with a syntax error), so
    /// the email is embedded as an escaped literal instead of a bound
    /// parameter.
    ///
    /// The statement returns one result set per table that has rows for this
    /// email (order is not guaranteed, and a table with no rows for this
    /// email gets no result set at all), so this dispatches each set by the
    /// table name its columns report. That is the same table name
    /// `PConRow`/`SchemaPolicy` resolution already keys off of for every
    /// other query in this file, so the rows come back with the same
    /// per-column policies a plain `SELECT * FROM <table>` would have
    /// produced.
    pub fn gdpr_get(
        &mut self,
        email: &str,
    ) -> (Vec<UserModel>, Vec<AnswerModel>, Vec<PresenterModel>) {
        self.ping_or_reconnect();

        let query = format!("GDPR GET users {}", mysql::Value::from(email).as_sql(true));
        let mut result = self
            .handle
            .query_iter(&query)
            .unwrap_or_else(|e| panic!("query '{}' failed ({})", query, e));

        let mut users = Vec::new();
        let mut answers = Vec::new();
        let mut presenters = Vec::new();
        loop {
            let table = result
                .columns()
                .as_ref()
                .first()
                .map(|c| c.table_str().into_owned());
            let table = match table {
                Some(table) => table,
                None => break,
            };
            match table.as_str() {
                "users" => {
                    for row in &mut result {
                        users.push(UserModel::from(row.unwrap()));
                    }
                }
                "answers" => {
                    for row in &mut result {
                        answers.push(AnswerModel::from(row.unwrap()));
                    }
                }
                "presenters" => {
                    for row in &mut result {
                        presenters.push(PresenterModel::from(row.unwrap()));
                    }
                }
                other => panic!("gdpr_get: unexpected table '{}' in GDPR GET response", other),
            }
        }
        (users, answers, presenters)
    }
}
