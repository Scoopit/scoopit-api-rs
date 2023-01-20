//! The types returned by the Scoop.it API
use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u64,
    pub name: String,
    pub short_name: String,
    pub url: String,
    pub bio: Option<String>,
    pub small_avatar_url: String,
    pub medium_avatar_url: String,
    pub avatar_url: String,
    pub large_avatar_url: String,
    pub curated_topics: Option<Vec<Topic>>,
    pub followed_topics: Option<Vec<Topic>>,
    pub premium_features: Option<BTreeSet<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Topic {
    pub id: u64,

    pub small_image_url: String,
    pub medium_image_url: String,
    pub image_url: String,
    pub large_image_url: String,
    pub description: Option<String>,
    pub name: String,
    pub short_name: String,
    pub url: String,
    pub lang: String,
    pub curated_post_count: u64,
    pub creator: Option<Box<User>>,
    pub pinned_post: Option<Post>,
    pub curated_posts: Option<Vec<Post>>,
    pub tags: Option<Vec<TopicTag>>,
    pub stats: Option<Stats>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicTag {
    pub tag: String,
    pub post_count: u32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub uv: i32,
    pub uvp: i32,
    pub v: i32,
    pub vp: i32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: i64,
    pub content: String,
    pub html_content: String,
    pub html_fragment: Option<String>,
    pub insight: Option<String>,
    pub html_insight: Option<String>,
    pub title: String,
    pub thanks_count: u32,
    pub reactions_count: u32,
    pub url: Option<String>,
    pub scoop_url: String,
    pub scoop_short_url: String,
    pub small_image_url: Option<String>,
    pub medium_image_url: Option<String>,
    pub image_url: Option<String>,
    pub large_image_url: Option<String>,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub image_size: Option<String>,
    pub image_position: Option<String>,
    pub tags: Option<Vec<String>>,
    pub comments_count: u32,
    pub page_views: Option<u32>,
    pub page_clicks: Option<u32>,
    pub author: Option<User>,
    pub is_user_suggestion: bool,
    pub suggested_by: Option<User>,
    pub twitter_author: Option<String>,
    pub publication_date: Option<i64>,
    pub curation_date: i64,
    pub topic_id: u64,
    pub topic: Option<Box<Topic>>,
    // note that this may change in the future to an opaque json object
    pub metadata: Option<HashMap<String, String>>,
}
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub users: Option<Vec<User>>,
    pub topics: Option<Vec<Topic>>,
    pub posts: Option<Vec<Post>>,
    pub total_found: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecipientsList {
    pub id: i64,
    pub name: String,
    pub emails: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionEngineType {
    ResearchContent,
    ContentMonitoring,
    Topic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuggestionEngine {
    pub id: i64,
    pub r#type: SuggestionEngineType,
    pub name: String,
    pub saved_searches: Option<Vec<SuggestionEngineSavedSearch>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SuggestionEngineSavedSearch {
    id: i64,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SourceTypeData {
    #[serde(rename_all = "camelCase")]
    AdvancedSearch {
        query: String,
    },
    #[serde(rename_all = "camelCase")]
    Webpage {
        webpage_url: Url,
    },
    #[serde(rename_all = "camelCase")]
    Rss {
        rss_url: Url,
    },
    #[serde(rename_all = "camelCase")]
    TwitterSearch {
        twitter_search_query: String,
    },
    #[serde(rename_all = "camelCase")]
    TwitterList {
        twitter_list_name: String,
        twitter_list_owner: String,
    },
    #[serde(rename_all = "camelCase")]
    TwitterFollowUser {
        twitter_user: String,
    },
    /// Search is encoded in the url of the source (this seems to be a scoop.it export bug)
    YoutubeSearch,
    /// url of the source is the facebook page
    FacebookPage,
    ScoopitUser,
    /// (topic)
    ScoopitTheme,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon_url: Url,
    /// in case of ScoopitUser & ScoopitTheme, the url is a relative url to https://www.scoop.it,
    /// so we cannot enforce an Option<Url> here
    pub url: Option<String>,
    #[serde(flatten)]
    pub source_type_data: SourceTypeData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicGroup {
    pub id: i64,
    pub name: String,
    pub url_name: String,
    pub user_editable: bool,
    pub topics: Vec<Topic>,
}
