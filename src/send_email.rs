use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SendEmailData {}

#[derive(Debug, Serialize)]
pub struct SendEmailResponse {}

pub fn send_email(data: SendEmailData) -> SendEmailResponse {
    SendEmailResponse {}
}
