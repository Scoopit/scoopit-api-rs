use scoopit_api::{
    serde_qs, types::SourceTypeData, BodyRequest, CreateSuggestionEngineSourceRequest,
    DeleteSuggestionEngineSourceRequest, GetSuggestionEngineSourcesRequest,
    UpdateSuggestionEngineSourceRequest,
};

#[test]
fn test_source_requests_serialization() {
    let get_source = GetSuggestionEngineSourcesRequest {
        suggestion_engine_id: 123,
    };
    assert_eq!(
        "",
        serde_qs::to_string(&get_source).expect("This must be serializable")
    );

    let delete_source = DeleteSuggestionEngineSourceRequest {
        suggestion_engine_id: 123,
        source_id: 456,
    };
    assert_eq!("se/123/sources/456", delete_source.endpoint());
    assert_eq!("", String::from_utf8_lossy(&delete_source.body().unwrap()));

    let update_source = UpdateSuggestionEngineSourceRequest {
        suggestion_engine_id: 123,
        source_id: 456,
        name: None,
    };
    assert_eq!("se/123/sources/456", update_source.endpoint());
    assert_eq!("", String::from_utf8_lossy(&update_source.body().unwrap()));

    let update_source = UpdateSuggestionEngineSourceRequest {
        suggestion_engine_id: 123,
        source_id: 456,
        name: Some("foobar".into()),
    };
    assert_eq!("se/123/sources/456", update_source.endpoint());
    assert_eq!(
        "name=foobar",
        String::from_utf8_lossy(&update_source.body().unwrap())
    );

    let create_twitter_source = CreateSuggestionEngineSourceRequest {
        suggestion_engine_id: 123,
        name: None,
        source_data: SourceTypeData::TwitterFollowUser {
            twitter_user: "bluxte".into(),
        },
    };
    assert_eq!("se/123/sources", create_twitter_source.endpoint());
    assert_eq!(
        "type=twitter_follow_user&twitterUser=bluxte",
        String::from_utf8_lossy(&create_twitter_source.body().unwrap())
    );
}
