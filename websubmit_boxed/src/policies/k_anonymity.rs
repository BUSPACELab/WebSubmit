use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};
use std::cmp;

// K-anonymity policy.
#[schema_policy(table = "agg_gender", column = 1)]
#[schema_policy(table = "agg_remote", column = 1)]
#[derive(Clone)]
pub struct KAnonymityPolicy {
    count: u64,
}

const MIN_K: u64 = 10;

impl SimplePolicy for KAnonymityPolicy {
    fn simple_name(&self) -> String {
        "KAnonymityPolicy".to_string()
    }

    fn simple_check(&self, _context: &UnprotectedContext, _reason: Reason) -> bool {
        self.count >= MIN_K
    }


    fn simple_join_direct(&mut self, other: &mut Self) {
        self.count = cmp::min(self.count, other.count);
    }
}

impl SchemaPolicy for KAnonymityPolicy {
    fn from_row(_table: &str, row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        KAnonymityPolicy {
            count: mysql::from_value(row[2].clone()),
        }
    }
}
