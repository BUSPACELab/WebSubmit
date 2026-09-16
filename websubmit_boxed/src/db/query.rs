use sesame::context::Context;
use sesame_mysql::{PConParams, PConRow};
use slog::warn;

use crate::db::MySqlBackend;
use crate::models::{
    AnswerModel, ConsentedAnswerModel, LectureModel, LectureWithQuestionCountsModel,
    PresenterModel, QuestionModel, UserModel,
};
use crate::policies::ContextData;

/// The column names of a query's WHERE clause.
///
/// Mirrors `Into<PConParams>` on the value side: a query passes the names here
/// and the matching values as params, and the two are zipped into
/// `WHERE <col> = ? AND ...`. Accepts `()`, a single name, or a tuple of names.
pub struct ColumnNames(Vec<String>);

impl ColumnNames {
    /// "SELECT * FROM <table>" with a WHERE clause for each named column.
    fn select_from(&self, table: &str) -> String {
        if self.0.is_empty() {
            return format!("SELECT * FROM {}", table);
        }
        format!(
            "SELECT * FROM {} WHERE {}",
            table,
            self.0
                .iter()
                .map(|c| format!("{} = ?", c))
                .collect::<Vec<String>>()
                .join(" AND ")
        )
    }
}

impl From<()> for ColumnNames {
    fn from(_: ()) -> Self {
        ColumnNames(vec![])
    }
}

impl From<&str> for ColumnNames {
    fn from(column: &str) -> Self {
        ColumnNames(vec![column.to_string()])
    }
}

macro_rules! column_names_tuple {
    ($($T:ident),+) => {
        impl<$($T: AsRef<str>),+> From<($($T,)+)> for ColumnNames {
            #[allow(non_snake_case)]
            fn from(columns: ($($T,)+)) -> Self {
                let ($($T,)+) = columns;
                ColumnNames(vec![$($T.as_ref().to_string()),+])
            }
        }
    };
}
column_names_tuple!(A);
column_names_tuple!(A, B);
column_names_tuple!(A, B, C);
column_names_tuple!(A, B, C, D);

/// Builds `query_<table>`: selects whole rows so that the schema policies line
/// up by column index, then converts each row into that table's module struct.
macro_rules! query_table {
    ($name:ident, $table:literal, $model:ty) => {
        pub fn $name<C: Into<ColumnNames>, P: Into<PConParams>>(
            &mut self,
            columns: C,
            values: P,
            context: Context<ContextData>,
        ) -> Vec<$model> {
            let columns: ColumnNames = columns.into();
            self.prep_exec(&columns.select_from($table), values, context)
                .into_iter()
                .map(Into::into)
                .collect()
        }
    };
}

impl MySqlBackend {
    query_table!(query_users, "users", UserModel);
    query_table!(query_lectures, "lectures", LectureModel);
    query_table!(query_questions, "questions", QuestionModel);
    query_table!(query_answers, "answers", AnswerModel);
    query_table!(query_presenters, "presenters", PresenterModel);

    query_table!(
        query_lectures_with_question_counts,
        "lectures_with_question_counts",
        LectureWithQuestionCountsModel
    );

    /// `consented_answers`'s `lec = ?` / `question_id = ?` are K9db matview
    /// key columns: they get resolved into the view at CREATE VIEW time, and
    /// a later lookup binds them by a literal value in the query text
    /// (matched by column name against the view's key schema), not by a
    /// bound `?` parameter the way `query_table!`'s WHERE clauses work.
    pub fn query_consented_answers(
        &mut self,
        lec: u64,
        question_id: u64,
        context: Context<ContextData>,
    ) -> Vec<ConsentedAnswerModel> {
        let sql = format!(
            "SELECT * FROM consented_answers WHERE lec = {} AND question_id = {}",
            lec, question_id
        );
        self.prep_exec(&sql, (), context)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Run a prepared query and collect its rows.
    ///
    /// Private: callers go through `query_<table>` so that every read is a whole
    /// row and the schema policies line up by column index.
    ///
    /// PConParams is not Clone in sesame, so a failed query cannot be replayed
    /// with the same params; the connection is checked up front instead.
    fn prep_exec<P: Into<PConParams>>(
        &mut self,
        sql: &str,
        params: P,
        context: Context<ContextData>,
    ) -> Vec<PConRow> {
        self.ping_or_reconnect();

        if !self.prep_stmts.contains_key(sql) {
            let stmt = self
                .handle
                .prep(sql)
                .expect(&format!("failed to prepare statement \'{}\'", sql));
            self.prep_stmts.insert(sql.to_owned(), stmt);
        }

        let params: PConParams = params.into();
        match self
            .handle
            .exec_iter(self.prep_stmts[sql].clone(), params, context)
        {
            Err(e) => {
                warn!(self.log, "query \'{}\' failed ({})", sql, e);
                panic!("query \'{}\' failed ({})", sql, e);
            }
            Ok(res) => res.map(|row| row.unwrap()).collect(),
        }
    }
}
