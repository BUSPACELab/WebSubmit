use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::State;

use sesame::pcon::PCon;
use sesame::context::Context;
use sesame_mysql::from_value;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::policy::{AnyPolicy, AnyPolicyClone, NoPolicy, Policy};
use sesame::verified::VerifiedRegion;
use sesame::SesameType;

use sesame_rocket::rocket::{
    post, PConCookie, PConCookieJar, PConForm, PConRedirect, PConRequest, PConRequestOutcome,
    FromPConForm, FromPConRequest, JsonResponse, OutputPConValue, ResponsePConJson,
};
use sesame::sandbox::execute_sandbox;

use crate::backend::MySqlBackend;
use crate::config::Config;
use crate::email;
use crate::policies::{ContextData, QueryableOnly};

use websubmit_boxed_sandboxes::hash;

// Errors that we may encounter when authenticating an ApiKey.
#[derive(Debug)]
pub(crate) enum ApiKeyError {
    Ambiguous,
    Missing,
    BackendFailure,
}

/// (username, apikey)
#[derive(SesameType, Clone)]
pub(crate) struct ApiKey {
    pub user: PCon<String, NoPolicy>,
    pub key: PCon<String, QueryableOnly>,
}

// Check API key against database.
pub(crate) fn check_api_key<P: Policy + Clone + 'static>(
    backend: &Arc<Mutex<MySqlBackend>>,
    key: &PCon<String, P>,
    context: Context<ContextData>,
) -> Result<PCon<String, NoPolicy>, ApiKeyError> {
    let mut bg = backend.lock().unwrap();
    let rs = bg.prep_exec(
        "SELECT * FROM users WHERE apikey = ?",
        (key.clone(),),
        context,
    );
    drop(bg);

    if rs.len() < 1 {
        Err(ApiKeyError::Missing)
    } else if rs.len() > 1 {
        Err(ApiKeyError::Ambiguous)
    } else if rs.len() == 1 {
        Ok(from_value(rs[0].get(0).unwrap()).unwrap())
    } else {
        Err(ApiKeyError::BackendFailure)
    }
}

// Auto construct ApiKey from every request using cookies.
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

#[derive(FromPConForm)]
pub(crate) struct ApiKeyRequest {
    email: PCon<String, NoPolicy>,
    gender: PCon<String, NoPolicy>,
    age: PCon<u32, NoPolicy>,
    ethnicity: PCon<String, NoPolicy>,
    is_remote: Option<PCon<bool, NoPolicy>>,
    education: PCon<String, NoPolicy>,
    consent: Option<PCon<bool, NoPolicy>>,
}

pub(crate) struct ApiKeyResponse {
    email: PCon<String, AnyPolicy>,
    apikey: PCon<String, AnyPolicy>,
}

impl ResponsePConJson for ApiKeyResponse {
    fn to_json(self) -> OutputPConValue {
        OutputPConValue::Object(HashMap::from([
            (String::from("email"), self.email.to_json()),
            (String::from("apikey"), self.apikey.to_json()),
        ]))
    }
}

#[derive(FromPConForm)]
pub(crate) struct ApiKeySubmit {
    key: PCon<String, NoPolicy>,
}

#[post("/", data = "<data>")]
pub(crate) fn generate(
    data: PConForm<ApiKeyRequest>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> JsonResponse<ApiKeyResponse, ContextData> {
    let pseudonym: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    // generate an API key from email address
    let hash: PCon<String, AnyPolicyClone> =
        execute_sandbox::<hash, _, _, _>((data.email.clone(), config.secret.clone()));

    // Check if request corresponds to admin or manager.
    let is_manager = data.email.verified(VerifiedRegion::new(|email: &String| {
        config.managers.contains(email)
    }));
    let is_admin = data.email.verified(VerifiedRegion::new(|email: &String| {
        config.admins.contains(email)
    }));

    // insert into MySql if not exists
    let mut bg = backend.lock().unwrap();
    bg.insert(
        "users",
        (
            data.email.clone(),
            hash.clone(),
            is_admin.to_owned_policy(),
            is_manager.to_owned_policy(),
            pseudonym,
            data.gender.clone(),
            data.age.clone(),
            data.ethnicity.clone(),
            match &data.is_remote {
                Some(is_remote) => is_remote.clone(),
                None => PCon::new(false, NoPolicy {}),
            },
            data.education.clone(),
            match &data.consent {
                Some(consent) => consent.clone(),
                None => PCon::new(false, NoPolicy {}),
            },
        ),
        context.clone(),
    );

    if config.send_emails {
        execute_critical(
            (data.email.clone(), hash.clone()),
            context.clone(),
            CriticalRegion::new(|(email, hash), _| {
                email::send(
                    bg.log.clone(),
                    "no-reply@csci2390-submit.cs.brown.edu".into(),
                    vec![email],
                    format!("{} API key", config.class),
                    format!("Your {} API key is: {}\n", config.class, hash),
                )
                .expect("failed to send API key email");
            }, 
            Signature{username: "corinnt", signature: "?"}), 
        ())
        .unwrap();
    }
    drop(bg);

    // return to user
    let ctx = ApiKeyResponse {
        email: data.email.clone().into_any_policy_no_clone(),
        apikey: hash.clone().into_any_policy_no_clone(),
    };

    JsonResponse::from((ctx, context))
}

#[post("/", data = "<data>")]
pub(crate) fn check(
    data: PConForm<ApiKeySubmit>,
    cookies: PConCookieJar<'_, '_>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    context: Context<ContextData>,
) -> PConRedirect {
    // check that the API key exists and set cookie
    let res = check_api_key(&*backend, &data.key, context.clone());
    match res {
        Err(ApiKeyError::BackendFailure) => {
            eprintln!("Problem communicating with MySql backend");
        }
        Err(ApiKeyError::Missing) => {
            eprintln!("No such API key");
        }
        Err(ApiKeyError::Ambiguous) => {
            eprintln!("Ambiguous API key");
        }
        Ok(_) => (),
    }

    if res.is_err() {
        PConRedirect::to2("/")
    } else {
        let cookie = PConCookie::build("apikey", data.into_inner().key)
            .path("/")
            .finish();
        cookies.add(cookie, context).unwrap();
        PConRedirect::to2("/leclist")
    }
}
