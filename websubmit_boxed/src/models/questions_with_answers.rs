use chrono::naive::NaiveDateTime;
use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};

use crate::policies::{AnswerAccessPolicy, UserEmailPolicy};

/// A row of the `questions_with_answers` view: one question paired with one
/// user, carrying that user's answer when they have given one.
///
/// The answer side is an outer join, so all of its columns are nullable. The
/// row still belongs to `user_email` either way, which is what the policies on
/// this view key off — an unanswered row has no `answer_email` to identify it.
pub struct QuestionWithAnswerModel {
    pub id: PCon<u64, NoPolicy>,
    pub lecture_id: PCon<u64, NoPolicy>,
    pub question_number: PCon<u64, NoPolicy>,
    pub question: PCon<String, NoPolicy>,
    pub user_email: PCon<String, UserEmailPolicy>,
    pub answer_id: PCon<Option<String>, UserEmailPolicy>,
    pub answer_email: PCon<Option<String>, UserEmailPolicy>,
    pub lec: PCon<Option<u64>, AnswerAccessPolicy>,
    pub question_id: PCon<Option<u64>, AnswerAccessPolicy>,
    pub answer: PCon<Option<String>, AnswerAccessPolicy>,
    pub submitted_at: PCon<Option<NaiveDateTime>, AnswerAccessPolicy>,
}

impl From<PConRow> for QuestionWithAnswerModel {
    fn from(row: PConRow) -> Self {
        QuestionWithAnswerModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            lecture_id: from_value(row.get(1).unwrap()).unwrap(),
            question_number: from_value(row.get(2).unwrap()).unwrap(),
            question: from_value(row.get(3).unwrap()).unwrap(),
            user_email: from_value(row.get(4).unwrap()).unwrap(),
            answer_id: from_value(row.get(5).unwrap()).unwrap(),
            answer_email: from_value(row.get(6).unwrap()).unwrap(),
            lec: from_value(row.get(7).unwrap()).unwrap(),
            question_id: from_value(row.get(8).unwrap()).unwrap(),
            answer: from_value(row.get(9).unwrap()).unwrap(),
            submitted_at: from_value(row.get(10).unwrap()).unwrap(),
        }
    }
}
