use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};

/// A row of the `lectures` table. No column carries a policy.
pub struct LectureModel {
    pub id: PCon<u64, NoPolicy>,
    pub label: PCon<String, NoPolicy>,
}

impl From<PConRow> for LectureModel {
    fn from(row: PConRow) -> Self {
        LectureModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            label: from_value(row.get(1).unwrap()).unwrap(),
        }
    }
}
