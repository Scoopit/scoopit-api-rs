use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::types::{Post, SearchResults, Topic, User};

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
    pub order: Option<TopicOrder>,
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
pub enum TopicOrder {
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
pub trait GetRequest: Serialize {
    /// The type returned by the Scoop.it API.
    ///
    /// It must be converible to this trait Output type.
    type Response: TryInto<Self::Output, Error = anyhow::Error> + DeserializeOwned;
    /// The type returned by the client
    type Output;

    fn endpoint() -> &'static str;
}

impl GetRequest for GetTopicRequest {
    type Response = TopicResponse;
    type Output = Topic;

    fn endpoint() -> &'static str {
        "topic"
    }
}
impl GetRequest for GetProfileRequest {
    type Response = UserResponse;
    type Output = User;

    fn endpoint() -> &'static str {
        "profile"
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

    fn endpoint() -> &'static str {
        "search"
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
    connected_user: Option<User>,
    error: Option<String>,
}

impl GetRequest for TestRequest {
    type Response = TestResponse;
    type Output = Option<User>;

    fn endpoint() -> &'static str {
        "test"
    }
}
impl TryFrom<TestResponse> for Option<User> {
    type Error = anyhow::Error;

    fn try_from(value: TestResponse) -> Result<Self, Self::Error> {
        if let Some(error) = value.error {
            Err(anyhow::anyhow!("Server returned an error: {}", error))
        } else {
            Ok(value.connected_user)
        }
    }
}
