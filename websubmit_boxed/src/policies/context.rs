use std::sync::{Arc, Mutex};

use rocket::State;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use sesame::SesameType;
use sesame::context::Context;
use sesame::pcon::PCon;
use sesame_rocket::rocket::{PConRequest, PConRequestOutcome, FromPConRequest};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::policies::{QueryableOnly, UserEmailPolicy};

// Custom developer defined payload attached to every context.
#[derive(SesameType, Clone)]
#[sesame_out_type(verbatim = [db, config])]
pub struct ContextData {
    pub user: Option<PCon<String, UserEmailPolicy>>,
    pub db: Arc<Mutex<MySqlBackend>>,
    pub config: Config,
}

// Build the custom payload for the context given HTTP request.
#[rocket::async_trait]
impl<'a, 'r> FromPConRequest<'a, 'r> for ContextData {
    type PConError = ();

    async fn from_pcon_request(
        request: PConRequest<'a, 'r>,
    ) -> PConRequestOutcome<Self, Self::PConError> {
        let db: &State<Arc<Mutex<MySqlBackend>>> = request.guard().await.unwrap();
        let config: &State<Config> = request.guard().await.unwrap();

        // Find user using ApiKey token from cookie.
        let apikey = request.cookies().get::<QueryableOnly>("apikey");
        let user = match apikey {
            None => None,
            Some(apikey) => {
                let apikey = apikey.value().to_owned();
                let mut bg = db.lock().unwrap();
                let res = bg.query_users(
                    "apikey",
                    (apikey,),
                    Context::empty(),
                );
                drop(bg);
                res.into_iter().next().map(|user| user.email)
            }
        };

        request
            .route()
            .and_then(|_| {
                Some(ContextData {
                    user,
                    db: db.inner().clone(),
                    config: config.inner().clone(),
                })
            })
            .into_outcome((Status::InternalServerError, ()))
    }
}
