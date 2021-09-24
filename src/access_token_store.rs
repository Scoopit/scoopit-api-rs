use std::{
    convert::TryInto,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use log::{debug, error};

use crate::{
    oauth::{AccessTokenRequest, AccessTokenResponse},
    AccessToken, ScoopitAPI,
};

struct AccessTokenRenewer {
    scoopit_api: ScoopitAPI,
    client: reqwest::Client,
    client_id: String,
    client_secret: String,
}

impl AccessTokenRenewer {
    async fn renew_token(&self, refresh_token: &str) -> anyhow::Result<AccessToken> {
        let new_access_token = self
            .client
            .post(self.scoopit_api.access_token_endpoint.clone())
            .form(&AccessTokenRequest {
                client_id: &self.client_id,
                client_secret: &self.client_secret,
                grant_type: "refresh_token",
                refresh_token: Some(refresh_token),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<AccessTokenResponse>()
            .await?;

        debug!("Got new token: {:?}", new_access_token);

        Ok(new_access_token.try_into()?)
    }
}

pub async fn authenticate_with_client_credentials(
    client: &reqwest::Client,
    scoopit_api: &ScoopitAPI,
    client_id: &str,
    client_secret: &str,
) -> anyhow::Result<AccessToken> {
    Ok(client
        .post(scoopit_api.access_token_endpoint.clone())
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
        .await?
        .try_into()?)
}

pub struct AccessTokenStore {
    renewer: Arc<AccessTokenRenewer>,
    access_token: Arc<RwLock<AccessToken>>,
}

impl AccessTokenStore {
    pub fn new(
        token: AccessToken,
        scoopit_api: ScoopitAPI,
        client: reqwest::Client,
        client_id: String,
        client_secret: String,
    ) -> Self {
        let access_token = Arc::new(RwLock::new(token));
        let renewer = Arc::new(AccessTokenRenewer {
            scoopit_api,
            client,
            client_id,
            client_secret,
        });
        AccessTokenStore::schedule_renewal(renewer.clone(), access_token.clone());
        Self {
            access_token,
            renewer,
        }
    }

    fn schedule_renewal(renewer: Arc<AccessTokenRenewer>, access_token: Arc<RwLock<AccessToken>>) {
        let renew_date = {
            let token = access_token.read().unwrap();
            // schedule renew 5 minutes after token expiry so we will be sure the
            // access token will get refreshed if it needs it, thus the refresh token will
            // also be refreshed (refresh token also expires, which forces us to keep the token
            // alive)
            token
                .renew
                .as_ref()
                .map(|renew| UNIX_EPOCH + Duration::from_secs(renew.expires_at + 300))
        };
        if let Some(renew_date) = renew_date {
            let wait_time = renew_date.duration_since(SystemTime::now()).ok();
            tokio::spawn(AccessTokenStore::renew_if_needed_log_error(
                renewer,
                access_token,
                wait_time,
            ));
        }
    }

    async fn renew_if_needed_log_error(
        renewer: Arc<AccessTokenRenewer>,
        access_token: Arc<RwLock<AccessToken>>,
        wait_time: Option<Duration>,
    ) {
        debug!("Access token renew scheduled!");
        if let Some(wait_time) = wait_time {
            tokio::time::sleep(wait_time).await;
        }
        if let Err(e) =
            AccessTokenStore::renew_token_if_needed(renewer.clone(), access_token.clone()).await
        {
            error!("Unable to renew access token! {}", e);
            tokio::time::sleep(Duration::from_secs(1)).await;
            AccessTokenStore::schedule_renewal(renewer, access_token);
        }
    }

    async fn renew_token_if_needed(
        renewer: Arc<AccessTokenRenewer>,
        access_token: Arc<RwLock<AccessToken>>,
    ) -> anyhow::Result<()> {
        let refresh_token = {
            let token = access_token.read().unwrap();
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

        let new_access_token = renewer.renew_token(&refresh_token).await?;

        {
            let mut token = access_token.write().unwrap();

            *token = new_access_token;
        }
        AccessTokenStore::schedule_renewal(renewer, access_token);

        Ok(())
    }

    pub async fn get_access_token(&self) -> anyhow::Result<String> {
        AccessTokenStore::renew_token_if_needed(self.renewer.clone(), self.access_token.clone())
            .await
            .context("Cannot renew access token!")?;
        Ok(self.access_token.read().unwrap().access_token.clone())
    }
}
