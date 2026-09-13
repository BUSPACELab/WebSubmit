use std::sync::{Arc, Mutex};

use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::State;

use sesame::pcon::PCon;
use sesame::context::Context;
use sesame_mysql::from_value;
use sesame::policy::NoPolicy;
use sesame_rocket::rocket::{PConRequest, PConRequestOutcome, FromPConRequest};
use sesame::SesameType;

use crate::backend::MySqlBackend;
use crate::config::Config;
use crate::policies::QueryableOnly;

// Custom developer defined payload attached to every context.
#[derive(SesameType, Clone)]
#[sesame_out_type(verbatim = [db, config])]
pub struct ContextData {
    pub user: Option<PCon<String, NoPolicy>>,
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
                let res = bg.prep_exec(
                    "SELECT * FROM users WHERE apikey = ?",
                    (apikey,),
                    Context::empty(),
                );
                drop(bg);
                if res.len() > 0 {
                    Some(from_value(res[0].get(0).unwrap()).unwrap())
                } else {
                    None
                }
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
