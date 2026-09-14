use sesame::SesameTypeOut;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};

use crate::config::Config;
use crate::policies::ContextData;

// Access control policy.
#[schema_policy(table = "users", column = 0)] // email
#[schema_policy(table = "answers", column = 0)] // id: embeds the owner's email
#[schema_policy(table = "answers", column = 1)] // email
#[schema_policy(table = "presenters", column = 2)] // email
#[schema_policy(table = "questions_with_answers", column = 4)] // user_email
#[schema_policy(table = "questions_with_answers", column = 5)] // answer_id: embeds the email
#[schema_policy(table = "questions_with_answers", column = 6)] // answer_email
#[derive(Clone)]
pub struct UserEmailPolicy {
    owner: Option<String>, // even if no owner, admins may access
}

impl SimplePolicy for UserEmailPolicy {
    fn simple_name(&self) -> String {
        "UserEmailPolicy".to_string()
    }

    fn simple_check(&self, context: &UnprotectedContext, _reason: Reason) -> bool {
        type ContextDataOut = <ContextData as SesameTypeOut>::Out;
        let context: &ContextDataOut = context.downcast_ref().unwrap();

        let user: &Option<String> = &context.user;
        let config: &Config = &context.config;

        // I am not an authenticated user. I cannot see any email addresses!
        if user.is_none() {
            return false;
        }

        // I am the owner of the email address.
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

impl SchemaPolicy for UserEmailPolicy {
    fn from_row(table: &str, row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        // The email column sits at a different index in each table.
        let email = match table {
            "users" => 0,
            "answers" => 1,
            "presenters" => 2,
            // The view's row belongs to the paired user, whose email is always
            // present, rather than to the (possibly NULL) answer email.
            "questions_with_answers" => 4,
            table => panic!("UserEmailPolicy registered on unexpected table '{}'", table),
        };
        UserEmailPolicy {
            owner: mysql::from_value(row[email].clone()),
        }
    }
}
