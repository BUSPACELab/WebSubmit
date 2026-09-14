use sesame::pcon::PCon;
use sesame::policy::NoPolicy;
use sesame_mysql::{from_value, PConRow};

use crate::policies::{QueryableOnly, UserEmailPolicy};

/// A row of the `users` table.
pub struct UserModel {
    pub email: PCon<String, UserEmailPolicy>,
    pub apikey: PCon<String, QueryableOnly>,
    pub is_admin: PCon<bool, NoPolicy>,
    pub consent: PCon<bool, NoPolicy>,
}

impl From<PConRow> for UserModel {
    fn from(row: PConRow) -> Self {
        UserModel {
            email: from_value(row.get(0).unwrap()).unwrap(),
            apikey: from_value(row.get(1).unwrap()).unwrap(),
            is_admin: from_value(row.get(2).unwrap()).unwrap(),
            consent: from_value(row.get(3).unwrap()).unwrap(),
        }
    }
}
