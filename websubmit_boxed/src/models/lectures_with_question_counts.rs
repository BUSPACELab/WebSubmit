use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};

/// A row of the `lectures_with_question_counts` view: every lecture with the
/// number of questions it has, including lectures that have none.
///
/// Views match no schema policy registration, so no column carries a policy.
pub struct LectureWithQuestionCountsModel {
    pub id: PCon<u64, NoPolicy>,
    pub label: PCon<String, NoPolicy>,
    pub U_c: PCon<u64, NoPolicy>,
}

impl From<PConRow> for LectureWithQuestionCountsModel {
    fn from(row: PConRow) -> Self {
        LectureWithQuestionCountsModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            label: from_value(row.get(1).unwrap()).unwrap(),
            U_c: from_value(row.get(2).unwrap()).unwrap(),
        }
    }
}
