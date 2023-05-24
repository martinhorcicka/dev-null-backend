use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};

use crate::config::Config;

pub fn send_email(data: SendEmailData, config: Config) -> SendEmailResponse {
    match send_email_impl(data, config) {
        Ok(_) => Default::default(),
        Err(error) => SendEmailResponse {
            status: Status::KO,
            message: Some(error),
        },
    }
}

fn send_email_impl(data: SendEmailData, config: Config) -> Result<(), String> {
    let credentials = get_credentials()?;
    let message = create_message(data, config)?;
    let mailer = create_mailer(credentials)?;

    mailer.send(&message).map_err(to_string)?;
    Ok(())
}

fn get_credentials() -> Result<Credentials, String> {
    let username = std::env::var("SENDER_EMAIL").map_err(to_string)?;
    let password = std::env::var("SENDER_PASSWORD").map_err(to_string)?;
    Ok(Credentials::new(username, password))
}

fn create_message(
    SendEmailData {
        first_name,
        last_name,
        email,
        body,
    }: SendEmailData,
    config: Config,
) -> Result<Message, String> {
    let sender_email = std::env::var("SENDER_EMAIL").map_err(to_string)?;
    Message::builder()
        .from(sender_email.parse().map_err(to_string)?)
        .to(config.receiver_email.parse().map_err(to_string)?)
        .subject(config.email_subject.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "From: {first_name} {last_name} <{email}>\n\n\n{body}"
        ))
        .map_err(to_string)
}

fn create_mailer(credentials: Credentials) -> Result<SmtpTransport, String> {
    let smtp_server = std::env::var("SENDER_SMTP_SERVER").map_err(to_string)?;
    Ok(SmtpTransport::relay(&smtp_server)
        .map_err(to_string)?
        .credentials(credentials)
        .build())
}

fn to_string<T>(val: T) -> String
where
    T: ToString,
{
    val.to_string()
}

#[derive(Debug, Deserialize)]
pub struct SendEmailData {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
    email: String,
    body: String,
}

#[derive(Debug, Default, Serialize)]
pub struct SendEmailResponse {
    status: Status,
    message: Option<String>,
}

#[derive(Debug, Default)]
enum Status {
    #[default]
    OK,
    KO,
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Status::OK => serializer.serialize_str("ok"),
            Status::KO => serializer.serialize_str("ko"),
        }
    }
}
