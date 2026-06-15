use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::{authentication::Credentials, AsyncSmtpTransport, AsyncSmtpTransportBuilder},
    AsyncTransport, Message, Tokio1Executor,
};

pub struct Mailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Mailbox,
    backend: String,
}

impl Mailer {
    pub fn from_env() -> anyhow::Result<Self> {
        let backend = std::env::var("EMAIL_BACKEND")
            .unwrap_or_else(|_| "django.core.mail.backends.smtp.EmailBackend".into());
        let from_raw = std::env::var("DEFAULT_FROM_EMAIL")
            .unwrap_or_else(|_| "ListaIsto <geral@listaisto.pt>".into());
        let from: Mailbox = from_raw.parse()?;

        if backend.contains("console") {
            return Ok(Self { transport: None, from, backend });
        }

        let host = std::env::var("EMAIL_HOST").unwrap_or_else(|_| "localhost".into());
        let port: u16 = std::env::var("EMAIL_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(587);
        let user = std::env::var("EMAIL_HOST_USER").unwrap_or_default();
        let pass = std::env::var("EMAIL_HOST_PASSWORD").unwrap_or_default();
        let use_ssl = std::env::var("EMAIL_USE_SSL")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let use_tls = std::env::var("EMAIL_USE_TLS")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let mut builder: AsyncSmtpTransportBuilder = if use_ssl {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?.port(port)
        } else if use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?.port(port)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host).port(port)
        };

        if !user.is_empty() {
            builder = builder.credentials(Credentials::new(user, pass));
        }

        Ok(Self {
            transport: Some(builder.build()),
            from,
            backend,
        })
    }

    pub async fn send_text(&self, to: &str, subject: &str, body: &str) -> anyhow::Result<()> {
        let to_box: Mailbox = to.parse()?;
        let msg = Message::builder()
            .from(self.from.clone())
            .to(to_box)
            .subject(subject)
            // Declare UTF-8 so accents (á, ç) and the em-dash (—) render in
            // every client, not just Gmail. lettre picks a safe transfer
            // encoding (quoted-printable/base64) for the non-ASCII body.
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_owned())?;

        match &self.transport {
            None => {
                tracing::info!(
                    "[console-email] subject={:?} to={} backend={}\n{}",
                    subject, to, self.backend, body
                );
                Ok(())
            }
            Some(t) => {
                t.send(msg).await?;
                Ok(())
            }
        }
    }
}
