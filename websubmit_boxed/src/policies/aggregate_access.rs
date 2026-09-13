use crate::config::Config;
use crate::policies::ContextData;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};
use sesame::SesameTypeOut;

// Aggregate access policy.
#[schema_policy(table = "agg_gender", column = 1)]
#[schema_policy(table = "agg_remote", column = 1)]
#[derive(Clone)]
pub struct AggregateAccessPolicy {
    sensitive: bool,
}

const SENSITIVE_TABLES: &'static [&'static str] = &["agg_gender"];

impl SimplePolicy for AggregateAccessPolicy {
    fn simple_name(&self) -> String {
        "AggregateAccessPolicy".to_string()
    }

    fn simple_check(&self, context: &UnprotectedContext, _reason: Reason) -> bool {
        type ContextDataOut = <ContextData as SesameTypeOut>::Out;
        let context: &ContextDataOut = context.downcast_ref().unwrap();

        let user: &Option<String> = &context.user;
        let config: &Config = &context.config;

        let user = user.as_ref().unwrap();
        if config.managers.contains(user) && !self.sensitive || config.admins.contains(user) {
            return true;
        }
        return false;
    }


    fn simple_join_direct(&mut self, other: &mut Self) {
        self.sensitive = self.sensitive || other.sensitive;
    }
}

impl SchemaPolicy for AggregateAccessPolicy {
    fn from_row(table: &str, _row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        AggregateAccessPolicy {
            sensitive: SENSITIVE_TABLES.contains(&table),
        }
    }
}
