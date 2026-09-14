use chrono::naive::NaiveDateTime;
use sesame::pcon::PCon;
use sesame_mysql::{from_value, PConRow};

use crate::policies::{AnswerAccessPolicy, UserEmailPolicy};

/// A row of the `answers` table.
///
/// `id` is the synthesised key "{email}-{question_id}", so it identifies the
/// submitter and is guarded like the email itself.
pub struct AnswerModel {
    pub id: PCon<String, UserEmailPolicy>,
    pub email: PCon<String, UserEmailPolicy>,
    pub lec: PCon<u64, AnswerAccessPolicy>,
    pub question_id: PCon<u64, AnswerAccessPolicy>,
    pub answer: PCon<String, AnswerAccessPolicy>,
    pub submitted_at: PCon<NaiveDateTime, AnswerAccessPolicy>,
}

impl From<PConRow> for AnswerModel {
    fn from(row: PConRow) -> Self {
        AnswerModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            email: from_value(row.get(1).unwrap()).unwrap(),
            lec: from_value(row.get(2).unwrap()).unwrap(),
            question_id: from_value(row.get(3).unwrap()).unwrap(),
            answer: from_value(row.get(4).unwrap()).unwrap(),
            submitted_at: from_value(row.get(5).unwrap()).unwrap(),
        }
    }
}
