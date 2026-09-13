use sesame::context::Context;
use sesame::policy::NoPolicy;
use sesame_rocket::rocket::{get, PConCookieJar, PConRedirect};
use rocket::State;
use std::sync::{Arc, Mutex};

use crate::apikey;
use crate::backend::MySqlBackend;
use crate::policies;

#[get("/")]
pub(crate) fn index(
    cookies: PConCookieJar<'_, '_>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<policies::ContextData>,
) -> PConRedirect {
    if let Some(cookie) = cookies.get::<NoPolicy>("apikey") {
        let apikey = cookie.into();
        match apikey::check_api_key(&*backend, &apikey, context) {
            Ok(_user) => PConRedirect::to2("/leclist"),
            Err(_) => PConRedirect::to2("/login"),
        }
    } else {
        PConRedirect::to2("/login")
    }
}
