//! # Rust client for www.scoop.it REST API
//!
//! The client uses `reqwest` with `rustls` to perform HTTP requests to www.scoop.it API.
use anyhow::Context;
use log::debug;
use oauth::{AccessTokenRequest, AccessTokenResponse};
pub use requests::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    convert::TryInto,
    fmt::Debug,
    sync::RwLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{header, Url};

/// default endpoint
pub const API_ENDPOINT: &'static str = "https://www.scoop.it/api/1/";
/// authorization endpoint
pub const AUTHORIZATION_ENDPOINT: &'static str = "https://www.scoop.it/oauth/authorize";
/// access token exchange endpoint
pub const ACCESS_TOKEN_ENDPOINT: &'static str = "https://www.scoop.it/oauth2/token";

mod oauth;
pub mod requests;
pub mod types;

/// Scoop.it API endpoints.
///
/// Use the `default()` method to get the default endpoints.
#[derive(Clone, Debug)]
pub struct ScoopitAPI {
    endpoint: String,
    authorization_endpoint: String,
    access_token_endpoint: String,
}

impl Default for ScoopitAPI {
    fn default() -> Self {
        Self {
            endpoint: API_ENDPOINT.to_string(),
            authorization_endpoint: AUTHORIZATION_ENDPOINT.to_string(),
            access_token_endpoint: ACCESS_TOKEN_ENDPOINT.to_string(),
        }
    }
}

impl ScoopitAPI {
    pub fn custom(
        endpoint: String,
        authorization_endpoint: String,
        access_token_endpoint: String,
    ) -> Self {
        Self {
            endpoint,
            authorization_endpoint,
            access_token_endpoint,
        }
    }
}

/// The client for the scoop.it API.
///
/// All requests done by the client are authenticated using an access token. The token
/// is automatically renewed be needed.
pub struct ScoopitAPIClient {
    scoopit_api: ScoopitAPI,
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
    access_token: RwLock<AccessToken>,
}

impl ScoopitAPIClient {
    /// Create a scoopit api client authenticated using client credentials authentication.
    ///
    /// Access token is automatically requested from scoop.it upon the creation of the client
    /// using the `client_credelentials` grant type. If it fails, an error is returned.
    pub async fn authenticate_with_client_credentials(
        scoopit_api: ScoopitAPI,
        client_id: &str,
        client_secret: &str,
    ) -> anyhow::Result<Self> {
        if !scoopit_api.endpoint.ends_with("/") {
            return Err(anyhow::anyhow!(
                "Endpoint must ends with a trailing slash: {}",
                scoopit_api.endpoint
            ));
        }
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(60))
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert(
                    header::USER_AGENT,
                    header::HeaderValue::from_static("reqwest (scoopit-api-rs)"),
                );
                headers
            })
            .build()?;

        let access_token = client
            .post(Url::parse(&scoopit_api.access_token_endpoint)?)
            .form(&AccessTokenRequest {
                client_id: client_id,
                client_secret: client_secret,
                grant_type: "client_credentials",
                refresh_token: None,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<AccessTokenResponse>()
            .await?;

        debug!("Creating client with access token: {:?}", access_token);

        Ok(Self {
            scoopit_api,
            client,
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            access_token: RwLock::new(access_token.try_into()?),
        })
    }

    async fn renew_token_if_needed(&self) -> anyhow::Result<()> {
        let refresh_token = {
            let token = self.access_token.read().unwrap();
            match &token.renew {
                Some(renew) => {
                    let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
                    if now_timestamp.as_secs() < renew.expires_at {
                        debug!("Access token: {}, no renew needed!", token.access_token);
                        // no renew needed
                        return Ok(());
                    } else {
                        debug!("Access token: {}, renew needed!", token.access_token);
                        renew.refresh_token.clone()
                    }
                }
                // no renew needed
                None => return Ok(()),
            }
        };
        // renew needed: lock lately to avoid having the lock guard being leaked in the future making
        // the client not Send

        let new_access_token = self
            .client
            .post(Url::parse(&self.scoopit_api.access_token_endpoint)?)
            .form(&AccessTokenRequest {
                client_id: &self.client_id,
                client_secret: &self.client_secret,
                grant_type: "refresh_token",
                refresh_token: Some(&refresh_token),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<AccessTokenResponse>()
            .await?;

        debug!("Got new token: {:?}", new_access_token);

        let mut token = self.access_token.write().unwrap();

        *token = new_access_token.try_into()?;

        Ok(())
    }

    async fn do_get<T: DeserializeOwned>(&self, url: Url) -> anyhow::Result<T> {
        self.renew_token_if_needed()
            .await
            .context("Cannot refresh access token!")?;
        Ok(self
            .client
            .get(url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.access_token.read().unwrap().access_token),
            )
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Perform a `GET` request to scoop.it API.
    ///
    /// The request must immplements the `GetRequest` trait which specifies
    /// serialization format of the response and conversion method to the actual
    /// output type.
    pub async fn get<R>(&self, request: R) -> anyhow::Result<R::Output>
    where
        R: GetRequest + Debug,
    {
        let mut url = Url::parse(&self.scoopit_api.endpoint)?.join(R::endpoint())?;
        url.set_query(Some(&serde_qs::to_string(&request)?));
        let response: R::Response = self
            .do_get(url)
            .await
            .with_context(|| format!("Cannot get from api, request: {:?}", request))?;

        response.try_into()
    }
}

/// Renewal data of an access token
pub struct AccessTokenRenew {
    expires_at: u64,
    refresh_token: String,
}
impl AccessTokenRenew {
    pub fn new(expires_at: u64, refresh_token: String) -> Self {
        Self {
            expires_at,
            refresh_token,
        }
    }
}

/// An access token
pub struct AccessToken {
    access_token: String,
    renew: Option<AccessTokenRenew>,
}

// we are only interested by the expiration
#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub exp: Option<u64>,
}

impl AccessToken {
    /// Creates a never expiring access token.
    ///
    /// This token will never be renewed. If it comes to expire, all requests using it will fail.
    pub fn new(access_token: String) -> Self {
        Self::with_renew(access_token, None)
    }

    /// Creates an access token.
    ///
    /// If `renew` is provided the access will automatically renewed if needed.
    pub fn with_renew(access_token: String, renew: Option<AccessTokenRenew>) -> Self {
        Self {
            access_token,
            renew,
        }
    }
}

impl TryFrom<AccessTokenResponse> for AccessToken {
    type Error = anyhow::Error;

    fn try_from(r: AccessTokenResponse) -> Result<Self, Self::Error> {
        let AccessTokenResponse {
            access_token,
            expires_in: _,
            refresh_token,
        } = r;
        let decoded = jsonwebtoken::dangerous_insecure_decode::<Claims>(&access_token)?;

        Ok(Self::with_renew(
            access_token,
            refresh_token
                .map::<anyhow::Result<AccessTokenRenew>, _>(|refresh_token| {
                    Ok(AccessTokenRenew {
                        expires_at: decoded.claims.exp.ok_or(anyhow::anyhow!(
                            "Refresh token provided but access token does not expires!"
                        ))?,
                        refresh_token,
                    })
                })
                .transpose()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        GetProfileRequest, GetTopicRequest, ScoopitAPIClient, SearchRequest, SearchRequestType,
    };

    async fn get_client() -> ScoopitAPIClient {
        let _ = dotenv::dotenv();
        let client_id = std::env::var("SCOOPIT_CLIENT_ID").unwrap();
        let client_secret = std::env::var("SCOOPIT_CLIENT_SECRET").unwrap();
        ScoopitAPIClient::authenticate_with_client_credentials(
            Default::default(),
            &client_id,
            &client_secret,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn get_profile() {
        let user = get_client()
            .await
            .get(GetProfileRequest {
                short_name: Some("pgassmann".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        println!("{:#?}", user);
    }

    #[tokio::test]
    async fn get_topic() {
        let topic = get_client()
            .await
            .get(GetTopicRequest {
                url_name: Some("sports-and-performance-psychology".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        println!("{:#?}", topic);
    }

    #[tokio::test]
    async fn search() {
        let client = get_client().await;
        println!(
            "{:#?}",
            client
                .get(SearchRequest {
                    query: "test".to_string(),
                    search_type: SearchRequestType::Post,
                    count: Some(3),
                    ..Default::default()
                })
                .await
                .unwrap()
        );
        println!(
            "{:#?}",
            client
                .get(SearchRequest {
                    query: "test".to_string(),
                    search_type: SearchRequestType::Topic,
                    count: Some(3),
                    ..Default::default()
                })
                .await
                .unwrap()
        );
        println!(
            "{:#?}",
            client
                .get(SearchRequest {
                    query: "test".to_string(),
                    search_type: SearchRequestType::User,
                    count: Some(3),
                    ..Default::default()
                })
                .await
                .unwrap()
        );
    }
}
