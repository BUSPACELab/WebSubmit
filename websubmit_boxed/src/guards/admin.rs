use rocket::State;
use rocket::http::Status;
use sesame::verified::VerifiedRegion;
use sesame_rocket::rocket::{FromPConRequest, PConRequest, PConRequestOutcome};

use crate::config::Config;
use crate::guards::apikey::ApiKey;

#[derive(Debug)]
pub(crate) enum AdminError {
    Unauthorized,
}

pub(crate) struct Admin {
    pub apikey: ApiKey,
}

#[rocket::async_trait]
impl<'a, 'r> FromPConRequest<'a, 'r> for Admin {
    type PConError = AdminError;

    async fn from_pcon_request(
        request: PConRequest<'a, 'r>,
    ) -> PConRequestOutcome<Self, Self::PConError> {
        // Not logged in, logged in but not an admin: both are Unauthorized.
        // Only a successful, admin ApiKey passes.
        match request.guard::<ApiKey>().await {
            PConRequestOutcome::Success(apikey) => {
                let cfg = request.guard::<&State<Config>>().await.unwrap();
                let user_email = apikey.user.clone();
                let admin = user_email.into_verified(VerifiedRegion::new(|user: String| {
                    if cfg.admins.contains(&user) {
                        Some(())
                    } else {
                        None
                    }
                }));

                match admin.fold_in() {
                    Some(_) => PConRequestOutcome::Success(Admin { apikey }),
                    None => PConRequestOutcome::Failure((Status::Unauthorized, AdminError::Unauthorized)),
                }
            }
            _ => PConRequestOutcome::Failure((Status::Unauthorized, AdminError::Unauthorized)),
        }
    }
}
