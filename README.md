# websubmit-rs: a simple class submission system

This is a fork of websubmit-rs, a web application for collecting student homework
submissions, written using [Rocket](https://rocket.rs), ported to run on
[Sesame](https://github.com/eth-sri/sesame)/[K9db](https://github.com/K9db/k9db) sandboxing and backend.

## Setup

The sandbox build needs `rust-src` on its pinned nightly:
```
rustup component add rust-src --toolchain nightly-2023-10-06
```

## Database

The application talks to [K9db](https://github.com/K9db/k9db), a sharded,
MySQL-protocol research database with owned-table/data-subject semantics. Run
it locally via Docker:
```
docker pull kinanbab/k9db:latest
docker run -d --name k9db -p 10001:10001 -v k9db-data:/var/lib/k9db kinanbab/k9db:latest
```

K9db does not support `DROP DATABASE`/`DROP TABLE`/`SHOW TABLES`, so schema
state persists in the `k9db-data` volume across restarts. If the app or its
tests fail with `Table exists`, reset the volume rather than looking for a
code regression:
```
docker stop k9db && docker rm k9db && docker volume rm k9db-data
docker run -d --name k9db -p 10001:10001 -v k9db-data:/var/lib/k9db kinanbab/k9db:latest
```

Once K9db is running, start the web application, which connects to the
database named by `db_name` in the configuration file:
```
WebSubmit$ cargo run --release -p websubmit_boxed
```
To create and initialize the database, set the `prime` variable in the configuration
file (see below).

The web interface will be served on `localhost:8000`, or on the `port` set in
the configuration file. Note that the templates included directly under
`websubmit_boxed/templates` are very basic placeholders; the `ds593/` directory
contains a themed, ready-to-use set of templates, resources and a sample
config (`ds593/sample-ds593.toml`) for the DS593 course deployment, which you
can point `template_dir`/`resource_dir` at instead of customizing your own.

By default, the application will read configuration file `sample-config.toml`,
but a real deployment will specify a custom config (`-c myconfig.toml`).
Configuration files are TOML files with the following format:
```
# short class ID (human readable)
class = "CSCI 2390"
# TCP port the web server listens on (optional; defaults to 8000)
port = 8000
# database server address, as `host` or `host:port` (optional; defaults to 127.0.0.1)
db_addr = "127.0.0.1:10001"
# MySQL database name
db_name = "myclass"
# MySQL database user
db_user = "root"
# MySQL database password
db_password = "password"
# list of email addresses whose API keys get admin access
admins = ["babman@bu.edu", "malte@cs.brown.edu"]
# list of email addresses who should receive notification emails
staff = ["babman@bu.edu", "malte@cs.brown.edu"]
# email domain that registrations are restricted to, written with its leading
# `@` (optional; omit or leave empty to accept any address). Checked server
# side by `/apikey/generate`, and by the login page itself. Addresses listed in
# `admins` or `staff` are exempt, so staff from another institution can still
# register.
email_domain = "@bu.edu"
# custom template directory
template_dir = "/path/to/templates"
# custom resource directory (e.g., for images, CSS, JS)
resource_dir = "/path/to/resources"
# a secret that will be hashed into user's API keys to make them unforgeable
secret = "SECRET"
# Anthropic API key, used by the admin automated lecture-summary analysis endpoint
anthropic_api_key = "ANTHROPIC_API_KEY"
# whether to send emails (set to false for development)
send_emails = false
# SMTP server used to send emails
smtp_server = "smtp.example.com"
# SMTP server port (587 submission/STARTTLS, 465 implicit TLS, 25 plain relay)
smtp_port = 587
# SMTP user; leave empty to use an unauthenticated relay
smtp_user = ""
# SMTP password
smtp_password = ""
# address that emails are sent from
smtp_from = "no-reply@example.com"
# whether to reset the db (set to false for production)
prime = true
```

If you omit `--release`, the web app will produce additional
debugging output.

## Testing

CI (`.github/workflows/ci.yml`) runs `cargo test --workspace` against a K9db
service container on every push/PR to `main`. To run the same tests locally,
start K9db as described above, then:
```
WebSubmit$ cargo test --workspace
```
The e2e tests share a single database and must run single-threaded; this is
already configured via `RUST_TEST_THREADS=1` in `.cargo/config.toml`.

The WASM sandbox (`websubmit_boxed_sandboxes`) is expensive to build and its
compiled `.so` is committed to the repo, so a plain `cargo test`/`cargo build`
will link the prebuilt sandbox instead of rebuilding it. Only rebuild it if
you change the sandboxed code, since doing so needs the pinned nightly,
`rust-src`, and WASI tooling.
