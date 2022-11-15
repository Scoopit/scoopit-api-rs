use scoopit_api::{
    types::Source, CreateSuggestionEngineSourceResponse, EmptyUpdateResponse,
    GetSuggestionEngineSourcesResponse, GetSuggestionEnginesResponse,
};

#[test]
fn test_get_suggestion_engines() {
    serde_json::from_str::<GetSuggestionEnginesResponse>(include_str!(
        "samples/suggestion_engines.json"
    ))
    .unwrap();
}

#[test]
fn test_get_sources() {
    serde_json::from_str::<GetSuggestionEngineSourcesResponse>(include_str!(
        "samples/sources.json"
    ))
    .unwrap();
}

#[test]
fn test_source_twitter_user() {
    serde_json::from_str::<Source>(include_str!("samples/source_twitter_user.json")).unwrap();
}

#[test]
fn test_update_resp() {
    assert!(
        serde_json::from_str::<EmptyUpdateResponse>(include_str!("samples/update_ok.json"))
            .unwrap()
            .is_ok()
    );
    assert!(
        serde_json::from_str::<EmptyUpdateResponse>(include_str!("samples/update_error.json"))
            .unwrap()
            .is_err()
    );
}

#[test]
fn test_create_source() {
    serde_json::from_str::<CreateSuggestionEngineSourceResponse>(include_str!(
        "samples/create_source.json"
    ))
    .unwrap();
}
