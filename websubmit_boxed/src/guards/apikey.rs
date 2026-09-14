use std::sync::{Arc, Mutex};

use rocket::State;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use sesame::SesameType;
use sesame::context::Context;
use sesame::pcon::PCon;
use sesame::policy::Policy;
use sesame_rocket::rocket::{PConCookie, FromPConRequest, PConRequest, PConRequestOutcome};

use crate::db::MySqlBackend;
use crate::policies::{ContextData, QueryableOnly, UserEmailPolicy};

// Errors that we may encounter when authenticating an ApiKey.
#[derive(Debug)]
pub(crate) enum ApiKeyError {
    Ambiguous,
    Missing,
}

#[derive(SesameType, Clone)]
pub(crate) struct ApiKey {
    pub user: PCon<String, UserEmailPolicy>,
    pub key: PCon<String, QueryableOnly>,
}

pub(crate) fn check_api_key<P: Policy + Clone + 'static>(
    backend: &Arc<Mutex<MySqlBackend>>,
    key: &PCon<String, P>,
    context: Context<ContextData>,
) -> Result<PCon<String, UserEmailPolicy>, ApiKeyError> {
    let mut bg = backend.lock().unwrap();
    let rs = bg.query_users(
        "apikey",
        (key.clone(),),
        context,
    );
    drop(bg);

    if rs.len() < 1 {
        Err(ApiKeyError::Missing)
    } else if rs.len() > 1 {
        Err(ApiKeyError::Ambiguous)
    } else {
        Ok(rs.into_iter().next().unwrap().email)
    }
}

#[rocket::async_trait]
impl<'a, 'r> FromPConRequest<'a, 'r> for ApiKey {
    type PConError = ApiKeyError;

    async fn from_pcon_request(
        request: PConRequest<'a, 'r>,
    ) -> PConRequestOutcome<Self, Self::PConError> {
        let context = request.guard().await.unwrap();
        let db: &State<Arc<Mutex<MySqlBackend>>> = request.guard().await.unwrap();

        request
            .cookies()
            .get::<QueryableOnly>("apikey")
            .and_then(|cookie: PConCookie<'_, QueryableOnly>| Some(cookie.into()))
            .and_then(
                |key: PCon<String, QueryableOnly>| match check_api_key(db, &key, context) {
                    Ok(user) => Some(ApiKey { user, key }),
                    Err(_) => None,
                },
            )
            .into_outcome((Status::Unauthorized, ApiKeyError::Missing))
    }
}
