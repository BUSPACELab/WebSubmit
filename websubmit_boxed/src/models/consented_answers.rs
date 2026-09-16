use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame::SesameType;
use sesame_mysql::{from_value, PConRow};

use crate::policies::AutomatedAnalysisPolicy;

/// A row of the `consented_answers` view: an answer from a consenting
/// student, alongside the lecture/question it belongs to.
///
/// Derives SesameType so a `Vec<ConsentedAnswerModel>` can be folded into one
/// unit and run through a single critical region (see routes/admin/analysis.rs).
#[derive(SesameType)]
pub struct ConsentedAnswerModel {
    pub answer: PCon<String, AutomatedAnalysisPolicy>,
    pub consent: PCon<bool, NoPolicy>,
    pub lec: PCon<u64, NoPolicy>,
    pub question_id: PCon<u64, NoPolicy>,
}

impl From<PConRow> for ConsentedAnswerModel {
    fn from(row: PConRow) -> Self {
        ConsentedAnswerModel {
            answer: from_value(row.get(0).unwrap()).unwrap(),
            consent: from_value(row.get(1).unwrap()).unwrap(),
            lec: from_value(row.get(2).unwrap()).unwrap(),
            question_id: from_value(row.get(3).unwrap()).unwrap(),
        }
    }
}
