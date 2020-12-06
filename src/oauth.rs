use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}
#[derive(Serialize)]
pub struct AccessTokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub grant_type: &'a str,
    pub refresh_token: Option<&'a str>,
}
