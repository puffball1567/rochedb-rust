// Parametrized security probe: build a connection from ORBP_* environment
// variables, attempt a put, and print SUCCESS or REJECT:<msg>. Used by the
// cross-driver security matrix harness.
use koutendb::{ConnectOptions, KoutenDb};

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

fn flag(key: &str) -> bool {
    env(key) == "1"
}

fn build() -> Result<KoutenDb, koutendb::Error> {
    let mut o = ConnectOptions::new(env("ORBP_PEERS"));
    if !env("ORBP_USER").is_empty() {
        o = o.username(env("ORBP_USER"));
    }
    if !env("ORBP_PASS").is_empty() {
        o = o.password(env("ORBP_PASS"));
    }
    if !env("ORBP_SECRET").is_empty() {
        o = o.secret_key(env("ORBP_SECRET"));
    }
    if flag("ORBP_TLS") {
        o = o.tls();
    }
    if !env("ORBP_CA").is_empty() {
        o = o.tls_ca_file(env("ORBP_CA"));
    }
    if !env("ORBP_SNI").is_empty() {
        o = o.tls_server_name(env("ORBP_SNI"));
    }
    if flag("ORBP_INSECURE") {
        o = o.danger_accept_invalid_certs();
    }
    o.connect()
}

fn main() {
    match build().and_then(|db| db.put_json("secure/demo", r#"{"probe":1}"#).map(|_| ())) {
        Ok(()) => println!("SUCCESS"),
        Err(e) => println!("REJECT:{}", e),
    }
}
