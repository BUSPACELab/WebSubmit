use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};
use sesame_rocket::render::PConRender;

/// A row of the `questions` table. No column carries a policy.
#[derive(PConRender)]
pub struct QuestionModel {
    pub id: PCon<u64, NoPolicy>,
    pub lecture_id: PCon<u64, NoPolicy>,
    pub question_number: PCon<u64, NoPolicy>,
    pub question: PCon<String, NoPolicy>,
}

impl From<PConRow> for QuestionModel {
    fn from(row: PConRow) -> Self {
        QuestionModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            lecture_id: from_value(row.get(1).unwrap()).unwrap(),
            question_number: from_value(row.get(2).unwrap()).unwrap(),
            question: from_value(row.get(3).unwrap()).unwrap(),
        }
    }
}
