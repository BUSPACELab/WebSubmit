use std::sync::{Arc, Mutex};

use rocket::http::Status;
use rocket::State;
use sesame::context::Context;
use sesame::critical::{execute_critical, CriticalRegion, Signature};
use sesame::pcon::PCon;
use sesame::policy::{AnyPolicyClone, NoPolicy};
use sesame::sandbox::execute_sandbox;
use sesame::verified::VerifiedRegion;
use sesame_rocket::render::PConRender;
use sesame_rocket::rocket::{post, FromPConForm, PConForm, PConTemplate};
use websubmit_boxed_sandboxes::hash;

use crate::config::Config;
use crate::db::MySqlBackend;
use crate::email;
use crate::policies::ContextData;

/// Longest email address accepted at registration.
const MAX_EMAIL_LENGTH: usize = 50;

// Email ends with configured domain (if any).
fn email_allowed(config: &Config, email: &str) -> bool {
    let suffix = match config.email_domain_suffix() {
        // No restriction configured.
        None => return true,
        Some(suffix) => suffix,
    };

    let email = email.trim().to_lowercase();
    email.ends_with(&suffix)
        || config.admins.iter().any(|a| a.trim().to_lowercase() == email)
        || config.staff.iter().any(|s| s.trim().to_lowercase() == email)
}

#[derive(FromPConForm)]
pub(crate) struct ApiKeyGenerateForm {
    email: PCon<String, NoPolicy>,
    consent: Option<PCon<bool, NoPolicy>>,
}

#[derive(PConRender)]
struct ApiKeyGeneratedRender {
    apikey_email: PCon<String, NoPolicy>,
}

#[post("/", data = "<data>")]
pub(crate) fn generate(
    data: PConForm<ApiKeyGenerateForm>,
    backend: &State<Arc<Mutex<MySqlBackend>>>,
    config: &State<Config>,
    context: Context<ContextData>,
) -> Result<PConTemplate, Status> {
    let email = data.email.clone().discard_box();
    if email.chars().count() > MAX_EMAIL_LENGTH || !email_allowed(config, &email) {
        return Err(Status::UnprocessableEntity);
    }

    // generate an API key from email address
    let hash: PCon<String, AnyPolicyClone> =
        execute_sandbox::<hash, _, _, _>((data.email.clone(), config.secret.clone()));

    let is_admin = data.email.verified(VerifiedRegion::new(|email: &String| {
        config.admins.contains(email)
    }));

    // The key is a deterministic hash of the email, so registering an address
    // again yields the same key: REPLACE refreshes the row rather than failing
    // on the primary key.
    let mut bg = backend.lock().unwrap();
    bg.replace(
        "users",
        (
            data.email.clone(),
            hash.clone(),
            is_admin.to_owned_policy(),
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
            CriticalRegion::new(
                |(email, hash), _| {
                    email::send(
                        bg.log.clone(),
                        config.inner(),
                        None,
                        vec![email],
                        format!("{} API key", config.class),
                        format!("Your {} API key is: {}\n", config.class, hash),
                    )
                    .expect("failed to send API key email");
                },
                Signature {
                    username: "corinnt",
                    signature: "?",
                },
            ),
            (),
        )
        .unwrap();
    } else {
        println!(
            "API key: {}",
            hash.clone()
                .specialize_policy::<NoPolicy>()
                .unwrap()
                .discard_box()
        );
    }
    drop(bg);

    let ctx = ApiKeyGeneratedRender {
        apikey_email: data.email.clone(),
    };
    Ok(PConTemplate::render("apikey/generated", &ctx, context).unwrap())
}
