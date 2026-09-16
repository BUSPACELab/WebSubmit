use sesame::SesameTypeOut;
use sesame::context::UnprotectedContext;
use sesame::policy::{Reason, SimplePolicy};
use sesame_mysql::{schema_policy, SchemaPolicy};

use crate::policies::ContextData;

/// Passed as the `arg` to `execute_critical`/`execute_verified` so this
/// policy's check can tell an automated-analysis read apart from any other
/// critical region that happens to touch an `answer` column.
pub const AUTOMATED_ANALYSIS_REASON: &str = "automated analysis";

// Guards the `answer` column of the `consented_answers` view. Unlike
// AnswerAccessPolicy (the base `answers` table), this only ever lets an
// admin declassify the answer, only for the automated-analysis critical
// region, and only for a student who consented.
#[schema_policy(table = "consented_answers", column = 0)] // answer
#[derive(Clone)]
pub struct AutomatedAnalysisPolicy {
    consent: bool,
}

impl AutomatedAnalysisPolicy {
    pub fn new(consent: bool) -> AutomatedAnalysisPolicy {
        AutomatedAnalysisPolicy { consent }
    }
}

impl SimplePolicy for AutomatedAnalysisPolicy {
    fn simple_name(&self) -> String {
        "AutomatedAnalysisPolicy".to_string()
    }

    fn simple_check(&self, context: &UnprotectedContext, reason: Reason) -> bool {
        // 1. The student who wrote this answer consented.
        if !self.consent {
            return false;
        }

        // 3. This declassification is happening for automated analysis,
        //    not some other critical region that happens to touch `answer`.
        let is_automated_analysis = match reason {
            Reason::Custom(any) => any
                .downcast_ref::<String>()
                .map_or(false, |r| r == AUTOMATED_ANALYSIS_REASON),
            _ => false,
        };
        if !is_automated_analysis {
            return false;
        }

        // 2. The requesting user is an admin.
        type ContextDataOut = <ContextData as SesameTypeOut>::Out;
        let context: &ContextDataOut = context.downcast_ref().unwrap();
        match &context.user {
            Some(user) => context.config.admins.contains(user),
            None => false,
        }
    }

    fn simple_join_direct(&mut self, other: &mut Self) {
        self.consent = self.consent && other.consent;
    }
}

impl SchemaPolicy for AutomatedAnalysisPolicy {
    fn from_row(table: &str, row: &Vec<mysql::Value>) -> Self
    where
        Self: Sized,
    {
        match table {
            // consented_answers: (answer, consent, lec, question_id)
            "consented_answers" => {
                AutomatedAnalysisPolicy::new(mysql::from_value(row[1].clone()))
            }
            table => panic!(
                "AutomatedAnalysisPolicy registered on unexpected table '{}'",
                table
            ),
        }
    }
}
