use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};

use crate::policies::UserEmailPolicy;

/// A row of the `presenters` table.
pub struct PresenterModel {
    pub id: PCon<u64, NoPolicy>,
    pub lecture_id: PCon<u64, NoPolicy>,
    pub email: PCon<String, UserEmailPolicy>,
}

impl From<PConRow> for PresenterModel {
    fn from(row: PConRow) -> Self {
        PresenterModel {
            id: from_value(row.get(0).unwrap()).unwrap(),
            lecture_id: from_value(row.get(1).unwrap()).unwrap(),
            email: from_value(row.get(2).unwrap()).unwrap(),
        }
    }
}
