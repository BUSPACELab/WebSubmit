use rocket::State;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
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
        let apikey = request.guard::<ApiKey>().await.unwrap();
        let cfg = request.guard::<&State<Config>>().await.unwrap();

        let user_email = apikey.user.clone();
        let admin = user_email.into_verified(VerifiedRegion::new(|user: String| {
            if cfg.admins.contains(&user) {
                Some(())
            } else {
                None
            }
        }));

        let admin = match admin.fold_in() {
            None => None,
            Some(_) => Some(Admin { apikey }),
        };
        admin.into_outcome((Status::Unauthorized, AdminError::Unauthorized))
    }
}
