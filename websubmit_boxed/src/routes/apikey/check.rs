use std::sync::{Arc, Mutex};

use rocket::http::SameSite;
use rocket::State;
use sesame::context::Context;
use sesame::pcon::PCon;

use sesame_rocket::rocket::{post, FromPConForm, PConCookie, PConCookieJar, PConForm, PConRedirect};
use time::Duration;

use crate::db::MySqlBackend;
use crate::guards::apikey::{check_api_key, ApiKeyError};
use crate::policies::{ContextData, QueryableOnly};

// A semester's worth of sessions, so a student logs in once and stays signed
// in for the course rather than every browser restart.
const SESSION_LIFETIME_DAYS: i64 = 120;

#[derive(FromPConForm)]
pub(crate) struct ApiKeyCheckForm {
    key: PCon<String, QueryableOnly>,
}

#[post("/", data = "<data>")]
pub(crate) fn check(
    data: PConForm<ApiKeyCheckForm>,
    cookies: PConCookieJar<'_, '_>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    // check that the API key exists and set cookie
    let res = check_api_key(&*backend, &data.key, context.clone());
    match res {
        Err(ApiKeyError::Missing) => {
            eprintln!("No such API key");
            return PConRedirect::to2("/");
        }
        Err(ApiKeyError::Ambiguous) => {
            eprintln!("Ambiguous API key");
            return PConRedirect::to2("/");
        },
        Ok(_) => (),
    }

    // Lax (rather than Rocket's own Strict default) so a direct link from
    // email or the course's LMS -- a normal top-level navigation onto this
    // site -- still carries the cookie; cross-site POSTs stay blocked.
    let cookie = PConCookie::build("apikey", data.into_inner().key)
        .path("/")
        .max_age(Duration::days(SESSION_LIFETIME_DAYS))
        .same_site(SameSite::Lax)
        .finish();
    cookies.add(cookie, context).unwrap();
    PConRedirect::to2("/leclist")
}
