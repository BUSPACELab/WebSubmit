# websubmit-rs: a simple class submission system

This is a fork for websubmit-rs, a web application for collecting student homework
submissions, written using [Rocket](https://rocket.rs) for a MySQL backend.

## Setup

The sandbox build needs `rust-src` on its pinned nightly:
```
rustup component add rust-src --toolchain nightly-2023-10-06
```

## Database

You need to run a MySQL server deployment.
Then you can run the web application, which connects to the MySQL database
named by `db_name` in the configuration file:
```
WebSubmit$ cargo run --release -p websubmit_boxed
```
To create and initialize the database, set the `prime` variable in the configuration
file (see below).

The web interface will be served on `localhost:8000`, or on the `port` set in
the configuration file. Note that the templates included in this repository are
very basic; in practice, you will want to customize the files in
`websubmit_boxed/templates`.

By default, the application will read configuration file `sample-config.toml`,
but a real deployment will specify a custom config (`-c myconfig.toml`).
Configuration files are TOML files with the following format:
```
# short class ID (human readable)
class = "CSCI 2390"
# TCP port the web server listens on (optional; defaults to 8000)
port = 8000
# database server address, as `host` or `host:port` (optional; defaults to 127.0.0.1)
db_addr = "127.0.0.1"
# MySQL database name
db_name = "myclass"
# list of staff email addresses (these users' API keys get admin access)
staff = ["malte@cs.brown.edu"]
# custom template directory
template_dir = "/path/to/templates"
# custom resource directory (e.g., for images, CSS, JS)
resource_dir = "/path/to/resources"
# a secret that will be hashed into user's API keys to make them unforgeable
secret = "SECRET"
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

