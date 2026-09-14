use mysql::Value;
use rocket::Request;
use rocket::http::Cookie;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};
use sesame_rocket::policy::FrontendPolicy;

#[derive(Clone)]
#[schema_policy(table = "users", column = 1)]
pub struct QueryableOnly {}

// Content of apikey column can only be accessed by:
//   1. SELECT query
impl SimplePolicy for QueryableOnly {
    fn simple_name(&self) -> String {
        "QueryableOnly".to_string()
    }

    fn simple_check(&self, _context: &UnprotectedContext, reason: Reason) -> bool {
        match reason {
            Reason::DB(query, _) => query.starts_with("SELECT"),
            Reason::Cookie("apikey") => true,
            _ => false,
        }
    }

    fn simple_join_direct(&mut self, _other: &mut Self) {
        // QueryableOnly carries no state; nothing to combine.
    }
}

impl FrontendPolicy for QueryableOnly {
    fn from_request<'a, 'r>(_request: &'a Request<'r>) -> Self {
        QueryableOnly {}
    }
    fn from_cookie<'a, 'r>(
        _name: &str,
        _cookie: &'a Cookie<'static>,
        _request: &'a Request<'r>,
    ) -> Self {
        QueryableOnly {}
    }
}

impl SchemaPolicy for QueryableOnly {
    fn from_row(_table: &str, _row: &Vec<Value>) -> Self
    where
        Self: Sized,
    {
        QueryableOnly {}
    }
}
