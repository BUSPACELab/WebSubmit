use crate::config::Config;
use crate::policies::ContextData;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};
use sesame::SesameTypeOut;

// ML training policy.
#[schema_policy(table = "ml_training", column = 0)]
#[schema_policy(table = "ml_training", column = 1)]
#[derive(Clone)]
pub struct MLTrainingPolicy {
    consent: bool,
}

impl SimplePolicy for MLTrainingPolicy {
    fn simple_name(&self) -> String {
        "MLTrainingPolicy".to_string()
    }

    fn simple_check(&self, context: &UnprotectedContext, _reason: Reason) -> bool {
        type ContextDataOut = <ContextData as SesameTypeOut>::Out;
        let context: &ContextDataOut = context.downcast_ref().unwrap();

        let user: &Option<String> = &context.user;
        let config: &Config = &context.config;

        let user = user.as_ref().unwrap();
        if config.managers.contains(user) && self.consent {
            return true;
        }
        return false;
    }


    fn simple_join_direct(&mut self, other: &mut Self) {
        self.consent = self.consent && other.consent;
    }
}

impl SchemaPolicy for MLTrainingPolicy {
    fn from_row(_table: &str, row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        MLTrainingPolicy {
            consent: mysql::from_value(row[2].clone()),
        }
    }
}
