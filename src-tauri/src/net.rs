//! Cliente HTTP compartilhado.
//!
//! - TLS nativo (store de certificados do SO — passa por inspeção HTTPS ex. Kaspersky).
//! - Sem proxy do sistema (proxy do Kaspersky quebrava as conexões).
//! - Força IPv4 (redes com IPv6 sem rota travam a conexão até o timeout).
//! - Timeout explícito.

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

pub fn client(timeout_secs: u64) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .use_native_tls()
        .no_proxy()
        .local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}
