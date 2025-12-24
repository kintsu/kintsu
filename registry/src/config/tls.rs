use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::io::{BufReader, Read};

/// TLS configuration for Cloudflare origin server setup.
/// See: https://developers.cloudflare.com/ssl/origin-configuration/
#[derive(Deserialize, Debug, Default)]
pub struct TlsConfig {
    #[serde(alias = "CERT_FILE")]
    pub cert_file: Option<String>,

    #[serde(alias = "KEY_FILE")]
    pub key_file: Option<String>,

    #[serde(alias = "CERTIFICATE")]
    pub certificate: Option<SecretString>,

    #[serde(alias = "KEY")]
    pub key: Option<SecretString>,

    #[serde(alias = "CLIENT_CA_FILE")]
    pub client_ca_file: Option<String>,

    #[serde(alias = "CLIENT_CA")]
    pub client_ca: Option<SecretString>,

    #[serde(default, alias = "REQUIRE_CLIENT_CERT")]
    pub require_client_cert: bool,
}

impl TlsConfig {
    pub fn is_configured(&self) -> bool {
        (self.cert_file.is_some() && self.key_file.is_some())
            || (self.certificate.is_some() && self.key.is_some())
    }

    pub fn load_cert_chain(
        &self
    ) -> crate::Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
        let reader = self
            .pem_reader(&self.cert_file, &self.certificate)
            .ok_or_else(|| crate::Error::TlsConfig("no certificate configured".into()))?;

        load_certs(reader)
    }

    pub fn load_private_key(&self) -> crate::Result<rustls::pki_types::PrivateKeyDer<'static>> {
        let reader = self
            .pem_reader(&self.key_file, &self.key)
            .ok_or_else(|| crate::Error::TlsConfig("no private key configured".into()))?;

        load_private_key(reader)
    }

    pub fn load_client_ca_roots(&self) -> crate::Result<rustls::RootCertStore> {
        let mut root_store = rustls::RootCertStore::empty();

        if let Some(reader) = self.pem_reader(&self.client_ca_file, &self.client_ca) {
            let certs = load_certs(reader)?;
            for cert in certs {
                root_store.add(cert).map_err(|e| {
                    crate::Error::TlsConfig(format!("failed to add CA cert: {}", e))
                })?;
            }
        } else {
            tracing::warn!(
                "no client CA configured - using system trust store; \
                 this may allow unauthorized clients to connect"
            );
            load_native_certs(&mut root_store)?;
        }

        Ok(root_store)
    }

    fn pem_reader(
        &self,
        file_path: &Option<String>,
        inline_pem: &Option<SecretString>,
    ) -> Option<PemSource> {
        if let Some(path) = file_path {
            Some(PemSource::File(path.clone()))
        } else {
            inline_pem
                .as_ref()
                .map(|s| PemSource::Inline(s.expose_secret().to_string()))
        }
    }
}

enum PemSource {
    File(String),
    Inline(String),
}

impl PemSource {
    fn into_reader(self) -> crate::Result<Box<dyn Read>> {
        match self {
            PemSource::File(path) => {
                let file = std::fs::File::open(&path).map_err(|e| {
                    crate::Error::TlsConfig(format!("failed to open {}: {}", path, e))
                })?;
                Ok(Box::new(file))
            },
            PemSource::Inline(pem) => Ok(Box::new(std::io::Cursor::new(pem.into_bytes()))),
        }
    }
}

fn load_certs(source: PemSource) -> crate::Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let reader = source.into_reader()?;
    let mut buf_reader = BufReader::new(reader);

    rustls_pemfile::certs(&mut buf_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::Error::TlsConfig(format!("failed to parse certificates: {}", e)))
}

fn load_private_key(source: PemSource) -> crate::Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let reader = source.into_reader()?;
    let mut buf_reader = BufReader::new(reader);

    rustls_pemfile::private_key(&mut buf_reader)
        .map_err(|e| crate::Error::TlsConfig(format!("failed to parse private key: {}", e)))?
        .ok_or_else(|| crate::Error::TlsConfig("no private key found".into()))
}

fn load_native_certs(root_store: &mut rustls::RootCertStore) -> crate::Result<()> {
    let native_certs = rustls_native_certs::load_native_certs();

    for err in native_certs.errors {
        tracing::warn!("error loading native cert: {}", err);
    }

    for cert in native_certs.certs {
        root_store
            .add(cert)
            .map_err(|e| crate::Error::TlsConfig(format!("failed to add native cert: {}", e)))?;
    }

    Ok(())
}
