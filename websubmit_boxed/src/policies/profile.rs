use crate::config::Config;
use crate::policies::ContextData;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};
use sesame::SesameTypeOut;

// Access control policy.
#[schema_policy(table = "users", column = 5)] // gender
#[schema_policy(table = "users", column = 6)] // age
#[schema_policy(table = "users", column = 7)] // ethnicity
#[derive(Clone)]
pub struct UserProfilePolicy {
    owner: Option<String>, // even if no owner, admins may access
}

impl SimplePolicy for UserProfilePolicy {
    fn simple_name(&self) -> String {
        "UserProfilePolicy".to_string()
    }

    fn simple_check(&self, context: &UnprotectedContext, _reason: Reason) -> bool {
        type ContextDataOut = <ContextData as SesameTypeOut>::Out;
        let context: &ContextDataOut = context.downcast_ref().unwrap();

        let user: &Option<String> = &context.user;
        let config: &Config = &context.config;

        // I am not an authenticated user. I cannot see any profiles!
        if user.is_none() {
            return false;
        }

        // I am the owner of the profile.
        let user = user.as_ref().unwrap();
        if let Some(owner) = &self.owner {
            if owner == user {
                return true;
            }
        }

        // I am an admin.
        if config.admins.contains(user) {
            return true;
        }

        return false;
    }


    fn simple_join_direct(&mut self, other: &mut Self) {
        if !self.owner.eq(&other.owner) {
            self.owner = None;
        }
    }
}

impl SchemaPolicy for UserProfilePolicy {
    fn from_row(_table: &str, row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        UserProfilePolicy {
            owner: mysql::from_value(row[0].clone()),
        }
    }
}
