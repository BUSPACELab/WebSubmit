use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rocket::State;
use sesame::context::Context;
use sesame_rocket::rocket::{get, PConCookieJar, PConRedirect, PConTemplate};

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::policies::{ContextData, QueryableOnly};

#[get("/")]
pub(crate) fn index(
    cookies: PConCookieJar<'_, '_>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    if let Some(cookie) = cookies.get::<QueryableOnly>("apikey") {
        let apikey = cookie.into();
        match crate::guards::apikey::check_api_key(&*backend, &apikey, context) {
            Ok(_user) => PConRedirect::to2("/leclist"),
            Err(_) => PConRedirect::to2("/login"),
        }
    } else {
        PConRedirect::to2("/login")
    }
}

#[get("/")]
pub(crate) fn login(config: &State<Config>, context: Context<ContextData>) -> PConTemplate {
    let mut ctx = HashMap::new();
    ctx.insert("CLASS_ID", config.class.clone());
    PConTemplate::render("login", &ctx, context).unwrap()
}

// Public: readable before registering, since it is what a visitor is meant to
// read before deciding whether to consent.
#[get("/")]
pub(crate) fn privacy(config: &State<Config>, context: Context<ContextData>) -> PConTemplate {
    let mut ctx = HashMap::new();
    ctx.insert("CLASS_ID", config.class.clone());
    PConTemplate::render("privacy", &ctx, context).unwrap()
}
