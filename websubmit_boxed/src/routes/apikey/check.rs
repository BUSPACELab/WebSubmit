use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame::pcon::PCon;

use sesame_rocket::rocket::{post, FromPConForm, PConCookie, PConCookieJar, PConForm, PConRedirect};

use crate::db::MySqlBackend;
use crate::guards::apikey::{check_api_key, ApiKeyError};
use crate::policies::{ContextData, QueryableOnly};

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

    let cookie = PConCookie::build("apikey", data.into_inner().key)
        .path("/")
        .finish();
    cookies.add(cookie, context).unwrap();
    PConRedirect::to2("/leclist")
}
