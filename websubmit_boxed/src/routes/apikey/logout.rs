use rocket::http::SameSite;
use sesame::context::Context;
use sesame::pcon::PCon;
use sesame_rocket::rocket::{get, PConCookie, PConCookieJar, PConRedirect};
use time::OffsetDateTime;

use crate::policies::{ContextData, QueryableOnly};

#[get("/")]
pub(crate) fn logout(cookies: PConCookieJar<'_, '_>, context: Context<ContextData>) -> PConRedirect {
    let expired = PConCookie::build("apikey", PCon::new(String::new(), QueryableOnly {}))
        .path("/")
        .same_site(SameSite::Lax)
        .expires(OffsetDateTime::unix_epoch())
        .finish();
    cookies.add(expired, context).unwrap();
    PConRedirect::to2("/login")
}
