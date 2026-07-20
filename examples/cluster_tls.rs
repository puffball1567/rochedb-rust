// Connect to a TLS-enabled koutend over the cluster wire protocol.
//
// Requires a KoutenDB core built with -d:ssl. Run against a TLS listener, e.g.
// the one from the core's scripts/cluster_tls_smoke.sh:
//
//   KOUTEN_PEERS=127.0.0.1:17651 KOUTEN_TLS_CA=/path/server.crt \
//   KOUTEN_USER=alice KOUTEN_PASSWORD=secret KOUTEN_SECRET_KEY=shared-secret \
//   cargo run --example cluster_tls

use koutendb::{ConnectOptions, ReadRingOptions};

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| fallback.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let peers = env_or("KOUTEN_PEERS", "127.0.0.1:17651");

    let mut options = ConnectOptions::new(peers)
        .username(env_or("KOUTEN_USER", "alice"))
        .password(env_or("KOUTEN_PASSWORD", "secret"))
        .secret_key(env_or("KOUTEN_SECRET_KEY", "shared-secret"));

    // Verifying against the server's own self-signed certificate keeps
    // certificate verification on, which is why this stays out of the
    // danger_accept_invalid_certs path below.
    if let Ok(ca) = std::env::var("KOUTEN_TLS_CA") {
        options = options.tls_ca_file(ca);
    } else {
        options = options.tls();
    }
    if let Ok(name) = std::env::var("KOUTEN_TLS_SERVER_NAME") {
        options = options.tls_server_name(name);
    }
    // Local smoke tests only — never against a production server.
    if std::env::var("KOUTEN_TLS_INSECURE").is_ok() {
        options = options.danger_accept_invalid_certs();
    }

    let db = options.connect()?;

    let id = db.put_json("secure/demo", r#"{"title":"tls smoke","ok":true}"#)?;
    println!("id={id}");

    let page = db.read_ring_json(
        "secure/demo",
        &ReadRingOptions::new()
            .filter_json(&format!(r#"{{"id":"{id}"}}"#))
            .selection("{ title ok }"),
    )?;
    println!("page={page}");
    Ok(())
}
