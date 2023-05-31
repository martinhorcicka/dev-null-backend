use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use serde::Deserialize;

use crate::{error::Error, response::Response};

pub fn send_email(data: SendEmailData) -> Response<()> {
    match send_email_impl(data) {
        Ok(_) => Response::ok(()),
        Err(error) => Response::err(error),
    }
}

fn send_email_impl(data: SendEmailData) -> Result<(), Error> {
    let credentials = get_credentials()?;
    let message = create_message(data)?;
    let mailer = create_mailer(credentials)?;

    mailer.send(&message)?;
    Ok(())
}

fn get_credentials() -> Result<Credentials, Error> {
    let username = std::env::var("SENDER_EMAIL")?;
    let password = std::env::var("SENDER_PASSWORD")?;
    Ok(Credentials::new(username, password))
}

fn create_message(
    SendEmailData {
        first_name,
        last_name,
        email,
        body,
    }: SendEmailData,
) -> Result<Message, Error> {
    let sender_email = std::env::var("SENDER_EMAIL")?;
    let receiver_email = std::env::var("RECEIVER_EMAIL")?;
    let email_subject = std::env::var("RECEIVER_EMAIL_SUBJECT")?;
    Message::builder()
        .from(sender_email.parse()?)
        .to(receiver_email.parse()?)
        .subject(email_subject)
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "From: {first_name} {last_name} <{email}>\n\n\n{body}"
        ))
        .map_err(|err| err.into())
}

fn create_mailer(credentials: Credentials) -> Result<SmtpTransport, Error> {
    let smtp_server = std::env::var("SENDER_SMTP_SERVER")?;
    Ok(SmtpTransport::relay(&smtp_server)?
        .credentials(credentials)
        .build())
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
