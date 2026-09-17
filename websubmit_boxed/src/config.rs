use std::convert::TryFrom;
use std::fs;
use std::io::{Error, ErrorKind, Read};

use toml;

#[derive(Debug, Clone)]
pub struct Config {
    /// Textual identifier for class
    pub class: String,
    /// TCP port the web server listens on
    pub port: u16,
    /// Database server address, as `host` or `host:port`
    pub db_addr: String,
    /// Database name
    pub db_name: String,
    /// Database user
    pub db_user: String,
    /// Database password
    pub db_password: String,
    /// System admin addresses
    pub admins: Vec<String>,
    /// Staff email addresses
    pub staff: Vec<String>,
    /// Email domain registrations are restricted to (e.g. "@bu.edu");
    /// `None` accepts any address
    pub email_domain: Option<String>,
    /// Web template directory
    pub template_dir: String,
    /// Web resource root directory
    pub resource_dir: String,
    /// Secret (for API key generation)
    pub secret: String,
    /// Anthropic API key, for the admin automated-analysis endpoint
    pub anthropic_api_key: String,
    /// Whether to send emails
    pub send_emails: bool,
    /// SMTP server used to send emails
    pub smtp_server: String,
    /// SMTP server port
    pub smtp_port: u16,
    /// SMTP user (empty for an unauthenticated relay)
    pub smtp_user: String,
    /// SMTP password
    pub smtp_password: String,
    /// Address that emails are sent from
    pub smtp_from: String,
    /// Whether to reset and prime db
    pub prime: bool,
}

impl Config {
    pub fn parse(path: &str) -> Result<Config, Error> {
        let mut f = fs::File::open(path)?;
        let mut buf = String::new();
        f.read_to_string(&mut buf)?;

        let value = match toml::Parser::new(&buf).parse() {
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "failed to parse config!",
                ));
            }
            Some(v) => v,
        };

        Ok(Config {
            class: value.get("class").unwrap().as_str().unwrap().into(),
            // Optional, so that configs written before this key existed keep
            // working; 8000 is Rocket's own default.
            port: match value.get("port") {
                None => 8000,
                Some(v) => {
                    let port = v.as_integer().expect("port must be an integer");
                    u16::try_from(port).expect("port must be between 0 and 65535")
                }
            },
            // Required: no default, so a config that omits it fails loudly
            // instead of silently talking to the wrong database. A port may be
            // included (e.g. "127.0.0.1:10001").
            db_addr: value
                .get("db_addr")
                .expect("db_addr is required")
                .as_str()
                .expect("db_addr must be a string")
                .into(),
            db_name: value.get("db_name").unwrap().as_str().unwrap().into(),
            db_user: value.get("db_user").unwrap().as_str().unwrap().into(),
            db_password: value.get("db_password").unwrap().as_str().unwrap().into(),
            admins: value
                .get("admins")
                .unwrap()
                .as_slice()
                .unwrap()
                .into_iter()
                .map(|v| v.as_str().unwrap().into())
                .collect(),
            staff: value
                .get("staff")
                .unwrap()
                .as_slice()
                .unwrap()
                .into_iter()
                .map(|v| v.as_str().unwrap().into())
                .collect(),
            // Optional: a config that omits it (or leaves it empty) accepts
            // any email domain, which is how configs written before this key
            // existed behave.
            email_domain: value.get("email_domain").map(|v| {
                v.as_str()
                    .expect("email_domain must be a string")
                    .to_string()
            }),
            template_dir: value.get("template_dir").unwrap().as_str().unwrap().into(),
            resource_dir: value.get("resource_dir").unwrap().as_str().unwrap().into(),
            secret: value.get("secret").unwrap().as_str().unwrap().into(),
            anthropic_api_key: value
                .get("anthropic_api_key")
                .expect("anthropic_api_key is required")
                .as_str()
                .expect("anthropic_api_key must be a string")
                .into(),
            send_emails: value.get("send_emails").unwrap().as_bool().unwrap().into(),
        smtp_server: value.get("smtp_server").unwrap().as_str().unwrap().into(),
        smtp_port: value.get("smtp_port").unwrap().as_integer().unwrap() as u16,
        smtp_user: value.get("smtp_user").unwrap().as_str().unwrap().into(),
        smtp_password: value.get("smtp_password").unwrap().as_str().unwrap().into(),
        smtp_from: value.get("smtp_from").unwrap().as_str().unwrap().into(),
            prime: value.get("prime").unwrap().as_bool().unwrap().into(),
        })
    }

    /// The suffix an address has to end in, or `None` if registration is
    /// unrestricted. This is also what the login page's own check reads.
    ///
    /// The config states the domain with its `@` (`"@bu.edu"`), so the value
    /// is already the suffix; it is only trimmed and lowercased here, to match
    /// addresses case insensitively.
    pub fn email_domain_suffix(&self) -> Option<String> {
        self.email_domain
            .as_deref()
            .map(str::trim)
            .filter(|domain| !domain.is_empty())
            .map(str::to_lowercase)
    }
}
