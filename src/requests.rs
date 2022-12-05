use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    fmt::Debug,
    str::FromStr,
};

use anyhow::anyhow;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    serde_qs,
    types::{
        Post, RecipientsList, SearchResults, Source, SourceTypeData, SuggestionEngine, Topic,
        TopicGroup, User,
    },
};

/// Get the profile of a user.
///
/// Maps parameters of https://www.scoop.it/dev/api/1/urls#user
///
/// Documentation of each field comes from the page above. Default values documented are used only
/// ff the field is not present (`None`), `Default` implementation for this struct may differ from
/// Scoop.it defaults to avoid retrieving the world while only looking at the user profile.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetProfileRequest {
    /// string optional - the shortName of the user to lookup - defaults to the current user
    pub short_name: Option<String>,
    /// long optional - the id of the user to lookup - defaults to the current user
    pub id: Option<String>,
    /// bool optional - default to false. returns or not stats for each returned topic
    pub get_stats: bool,
    /// bool optional - default to true. returns or not list of tags for each returned topic
    pub get_tags: bool,
    /// int optional - default to 0, number of curated posts to retrieve for each topic present in user data
    pub curated: Option<u32>,
    /// int optional - default to 0, number of curable posts to retrieve for each topic the current user is the curator (so it should not be specified if the "id" parameter is specified)
    pub curable: Option<u32>,
    /// int optional - default to 0, the maximum number of comments to retrieve for each curated post found in each topic present in user data
    pub ncomments: Option<u32>,
    /// bool optional - default to true. returns or not list of followed topics
    pub get_followed_topics: bool,
    /// bool optional - default to true. returns or not list of curated topics
    pub get_curated_topics: bool,
    ///timestamp optional - default to 0 (unix epoch). Filter curated topics by creation date.
    pub filter_curated_topics_by_creation_date_from: Option<u64>,
    ///timestamp optional - default to 2^63. Filter curated topics by creation date.
    pub filter_curated_topics_by_creation_date_to: Option<u64>,
    /// bool optional - default to true. returns or not creator of each returned topic
    pub get_creator: bool,
}

impl Default for GetProfileRequest {
    fn default() -> Self {
        // sane defaults
        Self {
            short_name: None,
            id: None,
            get_stats: false,           // no stats by default
            get_tags: false,            // no tags by default
            curated: Some(0),           // do not retrieve posts on curated topics
            curable: Some(0),           // do not retrieve suggestion on curated topics
            ncomments: Some(0),         // force no comments
            get_followed_topics: false, // no followed topics by default
            get_curated_topics: true,   // get curated topics by default
            filter_curated_topics_by_creation_date_from: None,
            filter_curated_topics_by_creation_date_to: None,
            get_creator: false,
        }
    }
}

/// Get a Topic.
///
/// Maps parameters of https://www.scoop.it/dev/api/1/urls#topic
///
/// Documentation of each field comes from the page above. Default values documented are used only
/// ff the field is not present (`None`), `Default` implementation for this struct may differ from
/// Scoop.it defaults to avoid retrieving the world while only looking at the user profile.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetTopicRequest {
    /// long required, unless 'urlName' is provided - the id of the topic to lookup
    pub id: Option<u64>,
    /// string required, unless 'id' is provided - the urlName of the topic to lookup
    pub url_name: Option<String>,
    /// int optional, default to 30 - number of curated posts to retrieve for this topic
    pub curated: Option<u32>,
    /// int optional, default to 0
    pub page: Option<u32>,
    /// int optional, default to 0 - for this topic, this parameter is ignored if the current user is not the curator of this topic
    pub curable: Option<u32>,
    /// int optional, default to 0 - for this topic, this parameter is ignored if the current user is not the curator of this topic - get a given page of curable posts
    pub curable_page: Option<u32>,
    /// string mandatory if "since" parameter is not specified - sort order of the curated posts, can be "tag" (see below), "search" (filter result on query "q" mandatory - see below), "curationDate", "user" (same order as seen in the scoop.it website)
    pub order: Option<GetTopicOrder>,
    /// string[] mandatory if "order"=="tag"
    pub tag: Option<Vec<String>>,
    ///  string mandatory if "order"=="search" - the query to use to search in the topic
    pub q: Option<String>,
    ///timestamp - only retrieve curated post newer than this timestamp
    pub since: Option<i64>,
    /// timestamp optional - used with "since" parameter, retrieve curated posts posts older then this timestamp
    pub to: Option<i64>,
    /// int optional, default to 100 - each curated post found in this topic
    pub ncomments: Option<u32>,
    /// boolean optional, default to false - if true, the response will include the scheduled posts
    pub show_scheduled: bool,
}
#[derive(Serialize, Debug)]
pub enum GetTopicOrder {
    #[serde(rename = "tag")]
    Tag,
    #[serde(rename = "search")]
    Search,
    #[serde(rename = "curationDate")]
    CurationDate,
    #[serde(rename = "user")]
    User,
}

impl Default for GetTopicRequest {
    fn default() -> Self {
        Self {
            id: None,
            url_name: None,
            curated: Some(30),
            page: None,
            curable: Some(0),
            curable_page: None,
            order: None,
            tag: None,
            q: None,
            since: None,
            to: None,
            ncomments: Some(100),
            show_scheduled: false,
        }
    }
}

/// Represents a `GET` request.
pub trait GetRequest: Serialize + Debug {
    /// The type returned by the Scoop.it API.
    ///
    /// It must be converible to this trait Output type.
    type Response: TryInto<Self::Output, Error = anyhow::Error> + DeserializeOwned;
    /// The type returned by the client
    type Output;

    fn endpoint(&self) -> Cow<'static, str>;
}

/// A request that does an update, by default the body is serialized as
/// `application/x-www-form-urlencoded` and the method is `POST`
pub trait UpdateRequest: Serialize + Debug {
    /// The type returned by the Scoop.it API.
    ///
    /// It must be convertible to this trait Output type.
    type Response: TryInto<Self::Output, Error = anyhow::Error> + DeserializeOwned;
    /// The type returned by the client
    type Output;

    fn endpoint(&self) -> Cow<'static, str>;

    /// the content type of the post request, by default `application/x-www-form-urlencoded`
    fn content_type() -> &'static str {
        "application/x-www-form-urlencoded; charset=utf-8"
    }

    /// The body as bytes, by default the type implementing this trait is serialized using serde_qs.
    fn body(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_qs::to_string(&self)?.into_bytes())
    }

    fn method(&self) -> Method {
        Method::POST
    }
}

impl GetRequest for GetTopicRequest {
    type Response = TopicResponse;
    type Output = Topic;

    fn endpoint(&self) -> Cow<'static, str> {
        "topic".into()
    }
}
impl GetRequest for GetProfileRequest {
    type Response = UserResponse;
    type Output = User;

    fn endpoint(&self) -> Cow<'static, str> {
        "profile".into()
    }
}

#[derive(Deserialize)]
pub struct TopicResponse {
    pub topic: Option<Topic>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct UserResponse {
    pub user: Option<User>,
    pub error: Option<String>,
}

impl TryFrom<UserResponse> for User {
    type Error = anyhow::Error;

    fn try_from(value: UserResponse) -> Result<Self, Self::Error> {
        if let Some(error) = value.error {
            Err(anyhow::anyhow!("Server returned an error: {}", error))
        } else {
            value
                .user
                .ok_or(anyhow::anyhow!("No user nor error in response body!"))
        }
    }
}
impl TryFrom<TopicResponse> for Topic {
    type Error = anyhow::Error;

    fn try_from(value: TopicResponse) -> Result<Self, Self::Error> {
        if let Some(error) = value.error {
            Err(anyhow::anyhow!("Server returned an error: {}", error))
        } else {
            value
                .topic
                .ok_or(anyhow::anyhow!("No user no topic in response body!"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum SearchRequestType {
    User,
    Topic,
    Post,
}

impl FromStr for SearchRequestType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(SearchRequestType::User),
            "topic" => Ok(SearchRequestType::Topic),
            "post" => Ok(SearchRequestType::Post),
            other => Err(anyhow::anyhow!("Invalid request type: {}", other)),
        }
    }
}

/// Perform a search.
///
/// Maps parameters of https://www.scoop.it/dev/api/1/urls#search
///
/// Documentation of each field comes from the page above. Default values documented are used only
/// ff the field is not present (`None`), `Default` implementation for this struct may differ from
/// Scoop.it defaults to avoid retrieving the world while only looking at the user profile.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    ///string - type of object searched: "user", "topic" or "post"
    #[serde(rename = "type")]
    pub search_type: SearchRequestType,
    /// string - the search query
    pub query: String,
    /// int optional, default to 50 - the number of item per page
    pub count: Option<u32>,
    /// int optional, default to 0 -the page number to return, the first page is 0
    pub page: Option<u32>,
    /// string optional, default to "en" - the language of the content to search into
    pub lang: Option<String>,
    /// long optional - the id of the topic to search posts into
    pub topic_id: Option<u32>,
    /// bool optional, default to true - returns or not list of tags for each returned topic / post. only for type="topic" or type="post"
    pub get_tags: bool,
    /// bool optional, default to true - returns or not creator of each returned topic. only for type="topic"
    pub get_creator: bool,
    /// bool optional, default to true - returns or not stats for each returned topic. only for type="topic"
    pub get_stats: bool,
    /// bool optional, default to true - returns or not tags for topic of each returned post. only for type="post"
    pub get_tags_for_topic: bool,
    /// bool optional, default to true - returns or not stats for topic of each returned post. only for type="post"
    pub get_stats_for_topic: bool,
}
impl Default for SearchRequest {
    fn default() -> Self {
        Self {
            search_type: SearchRequestType::Post,
            query: "".to_string(),
            count: Some(50),
            page: None,
            lang: None,
            topic_id: None,
            get_tags: false,
            get_creator: true,
            get_stats: false,
            get_tags_for_topic: false,
            get_stats_for_topic: false,
        }
    }
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub users: Option<Vec<User>>,
    pub topics: Option<Vec<Topic>>,
    pub posts: Option<Vec<Post>>,
    pub total_found: i32,
}

impl GetRequest for SearchRequest {
    type Response = SearchResponse;

    type Output = SearchResults;

    fn endpoint(&self) -> Cow<'static, str> {
        "search".into()
    }
}

impl TryFrom<SearchResponse> for SearchResults {
    type Error = anyhow::Error;

    fn try_from(value: SearchResponse) -> Result<Self, Self::Error> {
        let SearchResponse {
            users,
            topics,
            posts,
            total_found,
        } = value;
        Ok(SearchResults {
            users,
            topics,
            posts,
            total_found,
        })
    }
}

/// Get the list of recipients lists
///
/// See https://www.scoop.it/dev/api/1/urls#recipients-list
///
#[derive(Serialize, Debug, Default)]
pub struct GetRecipientsListRequest {
    _dummy: (),
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetRecipientsListResponse {
    list: Vec<RecipientsList>,
}

impl GetRequest for GetRecipientsListRequest {
    type Response = GetRecipientsListResponse;
    type Output = Vec<RecipientsList>;

    fn endpoint(&self) -> Cow<'static, str> {
        "recipients-list".into()
    }
}
impl TryFrom<GetRecipientsListResponse> for Vec<RecipientsList> {
    type Error = anyhow::Error;

    fn try_from(value: GetRecipientsListResponse) -> Result<Self, Self::Error> {
        Ok(value.list)
    }
}

/// Test authentication credentials.
///
/// https://www.scoop.it/dev/api/1/urls#test
#[derive(Serialize, Debug, Default)]
pub struct TestRequest {
    _dummy: (),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TestResponse {
    connected_user: Option<String>,
    error: Option<String>,
}

impl GetRequest for TestRequest {
    type Response = TestResponse;
    type Output = Option<String>;

    fn endpoint(&self) -> Cow<'static, str> {
        "test".into()
    }
}
impl TryFrom<TestResponse> for Option<String> {
    type Error = anyhow::Error;

    fn try_from(value: TestResponse) -> Result<Self, Self::Error> {
        if let Some(error) = value.error {
            Err(anyhow::anyhow!("Server returned an error: {}", error))
        } else {
            Ok(value.connected_user)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl UpdateRequest for LoginRequest {
    type Response = LoginResponse;

    type Output = LoginAccessToken;

    fn endpoint(&self) -> Cow<'static, str> {
        "login".into()
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LoginResponse {
    Ok {
        #[serde(rename = "accessToken")]
        access_token: LoginAccessToken,
    },
    Err {
        errors: Vec<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct LoginAccessToken {
    pub oauth_token: String,
    pub oauth_token_secret: String,
}

impl TryFrom<LoginResponse> for LoginAccessToken {
    type Error = anyhow::Error;

    fn try_from(value: LoginResponse) -> Result<Self, Self::Error> {
        match value {
            LoginResponse::Ok { access_token } => Ok(access_token),
            LoginResponse::Err { errors } => Err(anyhow!(
                "Unable to login with errors: {}",
                errors.join(", ")
            )),
        }
    }
}
/// Get the list of available suggestion engines
///
/// https://www.scoop.it/dev/api/1/urls#se
#[derive(Debug, Default, Serialize)]
pub struct GetSuggestionEnginesRequest {
    _dummy: (),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GetSuggestionEnginesResponse {
    Ok {
        suggestion_engines: Vec<SuggestionEngine>,
    },
    Err {
        error: String,
    },
}

impl GetRequest for GetSuggestionEnginesRequest {
    type Response = GetSuggestionEnginesResponse;

    type Output = Vec<SuggestionEngine>;

    fn endpoint(&self) -> Cow<'static, str> {
        "se".into()
    }
}

impl TryFrom<GetSuggestionEnginesResponse> for Vec<SuggestionEngine> {
    type Error = anyhow::Error;

    fn try_from(value: GetSuggestionEnginesResponse) -> Result<Self, Self::Error> {
        match value {
            GetSuggestionEnginesResponse::Ok { suggestion_engines } => Ok(suggestion_engines),
            GetSuggestionEnginesResponse::Err { error } => {
                Err(anyhow!("Server returned an error: {error}"))
            }
        }
    }
}

/// Get manual user sources of a suggestion engine.
///
/// https://www.scoop.it/dev/api/1/urls#se_sources
#[derive(Debug, Serialize)]
pub struct GetSuggestionEngineSourcesRequest {
    #[serde(skip)]
    pub suggestion_engine_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GetSuggestionEngineSourcesResponse {
    Ok { sources: Vec<Source> },
    Err { error: String },
}

impl GetRequest for GetSuggestionEngineSourcesRequest {
    type Response = GetSuggestionEngineSourcesResponse;

    type Output = Vec<Source>;

    fn endpoint(&self) -> Cow<'static, str> {
        format!("se/{}/sources", self.suggestion_engine_id).into()
    }
}

impl TryFrom<GetSuggestionEngineSourcesResponse> for Vec<Source> {
    type Error = anyhow::Error;

    fn try_from(value: GetSuggestionEngineSourcesResponse) -> Result<Self, Self::Error> {
        match value {
            GetSuggestionEngineSourcesResponse::Ok { sources } => Ok(sources),
            GetSuggestionEngineSourcesResponse::Err { error } => {
                Err(anyhow!("Server returned an error: {error}"))
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum EmptyUpdateResponse {
    Err { error: String },
    Ok {},
}

impl EmptyUpdateResponse {
    pub fn is_ok(&self) -> bool {
        if let EmptyUpdateResponse::Ok {} = &self {
            true
        } else {
            false
        }
    }
    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}

impl TryFrom<EmptyUpdateResponse> for () {
    type Error = anyhow::Error;

    fn try_from(value: EmptyUpdateResponse) -> Result<Self, Self::Error> {
        match value {
            EmptyUpdateResponse::Err { error } => Err(anyhow!("Server returned an error: {error}")),
            EmptyUpdateResponse::Ok {} => Ok(()),
        }
    }
}

/// Delete a manually source from a suggestion engine
///
/// https://www.scoop.it/dev/api/1/urls#se_sources_id
#[derive(Serialize, Debug)]
pub struct DeleteSuggestionEngineSourceRequest {
    #[serde(skip)]
    pub suggestion_engine_id: i64,
    #[serde(skip)]
    pub source_id: i64,
}

impl UpdateRequest for DeleteSuggestionEngineSourceRequest {
    type Response = EmptyUpdateResponse;

    type Output = ();

    fn endpoint(&self) -> Cow<'static, str> {
        format!(
            "se/{}/sources/{}",
            self.suggestion_engine_id, self.source_id
        )
        .into()
    }

    fn method(&self) -> Method {
        Method::DELETE
    }
}

/// Update a manually source from a suggestion engine
///
/// https://www.scoop.it/dev/api/1/urls#se_sources_id
#[derive(Serialize, Debug)]
pub struct UpdateSuggestionEngineSourceRequest {
    #[serde(skip)]
    pub suggestion_engine_id: i64,
    #[serde(skip)]
    pub source_id: i64,
    pub name: Option<String>,
}

impl UpdateRequest for UpdateSuggestionEngineSourceRequest {
    type Response = EmptyUpdateResponse;

    type Output = ();

    fn endpoint(&self) -> Cow<'static, str> {
        format!(
            "se/{}/sources/{}",
            self.suggestion_engine_id, self.source_id
        )
        .into()
    }
}

/// Create a suggestion engine source
///
/// https://www.scoop.it/dev/api/1/urls#se_sources_id
#[derive(Serialize, Debug)]
pub struct CreateSuggestionEngineSourceRequest {
    #[serde(skip)]
    pub suggestion_engine_id: i64,
    pub name: Option<String>,
    #[serde(flatten)]
    pub source_data: SourceTypeData,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CreateSuggestionEngineSourceResponse {
    Ok { source: Source },
    Err { error: String },
}
impl UpdateRequest for CreateSuggestionEngineSourceRequest {
    type Response = CreateSuggestionEngineSourceResponse;

    type Output = Source;

    fn endpoint(&self) -> Cow<'static, str> {
        format!("se/{}/sources", self.suggestion_engine_id).into()
    }

    fn method(&self) -> Method {
        Method::PUT
    }
}
impl TryFrom<CreateSuggestionEngineSourceResponse> for Source {
    type Error = anyhow::Error;

    fn try_from(value: CreateSuggestionEngineSourceResponse) -> Result<Self, Self::Error> {
        match value {
            CreateSuggestionEngineSourceResponse::Ok { source } => Ok(source),
            CreateSuggestionEngineSourceResponse::Err { error } => {
                Err(anyhow!("Server returned an error: {error}"))
            }
        }
    }
}

/// Get the data about a topic group
///
/// https://www.scoop.it/dev/api/1/urls#topicGroup
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetTopicGroupRequest {
    pub url_name: String,
    /// Some apps may be able to specify company id. (privileged apps)
    pub company_id: Option<i64>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum GetTopicGroupResponse {
    #[serde(rename_all = "camelCase")]
    Ok {
        topic_group: TopicGroup,
    },
    Err {
        error: String,
    },
}

impl GetRequest for GetTopicGroupRequest {
    type Response = GetTopicGroupResponse;

    type Output = TopicGroup;

    fn endpoint(&self) -> Cow<'static, str> {
        "topic-group".into()
    }
}

impl TryFrom<GetTopicGroupResponse> for TopicGroup {
    type Error = anyhow::Error;

    fn try_from(value: GetTopicGroupResponse) -> Result<Self, Self::Error> {
        match value {
            GetTopicGroupResponse::Ok { topic_group } => Ok(topic_group),
            GetTopicGroupResponse::Err { error } => {
                Err(anyhow!("Server returned an error: {error}"))
            }
        }
    }
}

/// Get the data about a topic group
///
/// https://www.scoop.it/dev/api/1/urls#compilation
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetCompilationRequest {
    ///  method used for sorting posts (GetCompilationSort::Rss if not specified)
    pub sort: Option<GetCompilationSort>,
    ///  a list of topic ids that will be used to create the compilation
    pub topic_ids: Option<Vec<i64>>,
    /// create the compilation from topics in this topic group
    pub topic_group_id: Option<i64>,
    /// no posts older than this timestamp will be returned (in millis from unix epoch)
    pub since: Option<i64>,
    ///maximum number of posts to return
    pub count: Option<u32>,
    /// page number of posts to retrieve
    pub page: Option<u32>,
    /// the maximum number of comments to retrieve for each returned post
    pub ncomments: Option<u32>,
    // return the list of tags for each returned post.
    pub get_tags: Option<bool>,
    /// return tags for topic of each returned post
    pub get_tags_for_topic: Option<bool>,
    /// return stats for topic of each returned post
    pub get_stats_for_topic: Option<bool>,
}

#[derive(Serialize, Debug)]
pub enum GetCompilationSort {
    /// posts are ordered like in the RSS feed
    #[serde(rename = "rss")]
    Rss,
    /// posts are ordered like in the "My followed scoops" tab in a scoop.it user profile
    #[serde(rename = "timeline")]
    Timeline,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum GetCompilationResponse {
    Ok { posts: Vec<Post> },
    Err { error: String },
}

impl GetRequest for GetCompilationRequest {
    type Response = GetCompilationResponse;

    type Output = Vec<Post>;

    fn endpoint(&self) -> Cow<'static, str> {
        "compilation".into()
    }
}

impl TryFrom<GetCompilationResponse> for Vec<Post> {
    type Error = anyhow::Error;

    fn try_from(value: GetCompilationResponse) -> Result<Self, Self::Error> {
        match value {
            GetCompilationResponse::Ok { posts } => Ok(posts),
            GetCompilationResponse::Err { error } => {
                Err(anyhow!("Server returned an error: {error}"))
            }
        }
    }
}
