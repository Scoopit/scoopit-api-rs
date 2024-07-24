//! # Rust client for www.scoop.it REST API
//!
//! The client uses `reqwest` with `rustls` to perform HTTP requests to www.scoop.it API.
use anyhow::Context;
use jsonwebtokens::raw::TokenSlices;
use log::debug;
use oauth::AccessTokenResponse;
pub use requests::*;
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, convert::TryInto, fmt::Debug, time::Duration};

use reqwest::{header, RequestBuilder, Url};

// reexport crates
pub use reqwest;
pub use url;

mod access_token_store;
mod oauth;
pub mod requests;
pub mod types;
// Note we are using a very hacked slimmed&vendored version of serde_qs to allow serializing Vec in form of
// vec=foo&vec=bar&vec=baz instead of regular serde_qs vec[1]=foo&vec[2]=bar&vec[3]=baz
pub mod serde_qs;

pub mod error;

pub use access_token_store::AccessTokenStore;

/// Scoop.it API endpoints.
///
/// Use the `default()` method to get the default endpoints.
#[derive(Clone, Debug)]
pub struct ScoopitAPI {
    endpoint: Url,
    authorization_endpoint: Url,
    access_token_endpoint: Url,
}

impl Default for ScoopitAPI {
    fn default() -> Self {
        Self::custom(Url::parse("https://www.scoop.it").unwrap()).unwrap()
    }
}

impl ScoopitAPI {
    pub fn custom(base_url: Url) -> anyhow::Result<Self> {
        Ok(Self {
            endpoint: base_url.join("/api/1/")?,
            authorization_endpoint: base_url.join("/oauth/authorize")?,
            access_token_endpoint: base_url.join("/oauth2/token")?,
        })
    }

    pub fn with_endpoint(self, endpoint: Url) -> Self {
        Self { endpoint, ..self }
    }
}

/// The client for the scoop.it API.
///
/// All requests done by the client are authenticated using an access token. The token
/// is automatically renewed be needed.
pub struct ScoopitAPIClient {
    scoopit_api: ScoopitAPI,
    client: reqwest::Client,
    access_token: AccessTokenStore,
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
        let client = ScoopitAPIClient::create_client()?;

        let access_token = access_token_store::authenticate_with_client_credentials(
            &client,
            &scoopit_api,
            client_id,
            client_secret,
        )
        .await?;

        debug!("Creating client with access token: {:?}", access_token);

        Ok(Self {
            access_token: AccessTokenStore::new(
                access_token,
                scoopit_api.clone(),
                client.clone(),
                client_id.to_string(),
                client_secret.to_string(),
            ),
            scoopit_api,
            client,
        })
    }

    pub fn new(
        scoopit_api: ScoopitAPI,
        access_token_store: AccessTokenStore,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            access_token: access_token_store,
            client: ScoopitAPIClient::create_client()?,
            scoopit_api,
        })
    }

    fn create_client() -> anyhow::Result<reqwest::Client> {
        Ok(reqwest::ClientBuilder::new()
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
            .build()?)
    }

    async fn do_request<T: DeserializeOwned>(
        &self,
        request: RequestBuilder,
    ) -> Result<T, error::Error> {
        let json = request
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.access_token.get_access_token().await?),
            )
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        debug!("Received response {json}");
        Ok(serde_json::from_str::<T>(&json)?)
    }

    /// Perform a `GET` request to scoop.it API.
    ///
    /// The request must immplements the `GetRequest` trait which specifies
    /// serialization format of the response and conversion method to the actual
    /// output type.
    pub async fn get<R>(&self, request: R) -> Result<R::Output, error::Error>
    where
        R: GetRequest + Debug,
    {
        let mut url = self
            .scoopit_api
            .endpoint
            .join(request.endpoint().as_ref())
            .context("Cannot build the url")?;
        url.set_query(Some(
            &serde_qs::to_string(&request).context("Cannot build the url")?,
        ));
        let response: R::Response = self.do_request(self.client.get(url)).await?;

        response.try_into().map_err(error::Error::from)
    }

    /// Perform a request with a triggers an update (or an action) to scoop.it API.
    ///
    /// The request must implements the `UpdateRequest` trait.
    pub async fn update<R>(&self, request: R) -> Result<R::Output, error::Error>
    where
        R: UpdateRequest + Debug,
    {
        let url = self
            .scoopit_api
            .endpoint
            .join(request.endpoint().as_ref())
            .context("Cannot build the url")?;

        let response: R::Response = self
            .do_request(
                self.client
                    .request(request.method(), url)
                    .header(CONTENT_TYPE, R::content_type())
                    .body(request.body()?),
            )
            .await?;

        response.try_into().map_err(error::Error::from)
    }
}

/// Renewal data of an access token
#[derive(Debug)]
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
#[derive(Debug)]
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
        let exp = {
            let TokenSlices { claims, .. } = jsonwebtokens::raw::split_token(&access_token)?;
            let json_claims = jsonwebtokens::raw::decode_json_token_slice(claims)?;
            serde_json::from_value::<Claims>(json_claims)?.exp
        };

        Ok(Self::with_renew(
            access_token,
            refresh_token
                .map::<anyhow::Result<AccessTokenRenew>, _>(|refresh_token| {
                    Ok(AccessTokenRenew {
                        expires_at: exp.ok_or(anyhow::anyhow!(
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
        GetProfileRequest, GetTopicOrder, GetTopicRequest, ScoopitAPIClient, SearchRequest,
        SearchRequestType, TestRequest,
    };

    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Setup function that is only run once, even if called multiple times.
    fn setup_logger() {
        INIT.call_once(|| {
            env_logger::init();
        });
    }

    async fn get_client() -> ScoopitAPIClient {
        let _ = dotenv::dotenv();
        setup_logger();
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
        let client = get_client().await;
        let user = client
            .get(GetProfileRequest {
                short_name: Some("pgassmann".to_string()),
                ..Default::default()
            })
            .await;

        println!("{:#?}", user.unwrap());

        assert!(client
            .get(GetProfileRequest {
                short_name: Some("pgassmann-a-profile-that-should-not-exists".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err()
            .is_not_found());
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

        let topic = get_client()
            .await
            .get(GetTopicRequest {
                url_name: Some("sports-and-performance-psychology".to_string()),
                order: Some(GetTopicOrder::User),
                ..Default::default()
            })
            .await
            .unwrap();
        println!("{:#?}", topic);
    }

    #[tokio::test]
    async fn get_topic_with_tags() {
        let client = get_client().await;

        let topic = client
            .get(GetTopicRequest {
                url_name: Some("best-of-photojournalism".to_string()),
                order: Some(GetTopicOrder::Tag),
                tag: Some(vec!["afghanistan".to_string()]),
                ..Default::default()
            })
            .await
            .unwrap();
        println!("{:#?}", topic);

        assert!(client
            .get(GetTopicRequest {
                url_name: Some("best-of-photojournalism-that-must-not-exists-yolo".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err()
            .is_not_found());
    }

    #[tokio::test]
    async fn get_test() {
        let response = get_client()
            .await
            .get(TestRequest::default())
            .await
            .unwrap();
        println!("{:#?}", response);
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
    /*
    #[tokio::test]
    async fn login() {
        let client = get_client().await;

        let result = client
            .post(LoginRequest {
                email: std::env::var("SCOOPIT_TEST_EMAIL").unwrap(),
                password: std::env::var("SCOOPIT_TEST_PWD").unwrap(),
            })
            .await
            .unwrap();

        println!("{:#?}", result)
    }
    */
}
