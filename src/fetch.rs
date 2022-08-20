use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use reqwest::Url;
use serde_json::Value;
use std::collections::HashMap;

const AUTHORIZATION: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
const PRIVATE_API_BASE: &str = "https://twitter.com/i/api/";

// TODO: token is a string of numbers, so better to store as integer (?)
static GUEST_TOKEN: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

/// get "x-guest-token" for subsequent requests
pub async fn new_guest_token() -> String {
  let url = "https://api.twitter.com/1.1/guest/activate.json";
  let client = reqwest::Client::new();
  let text = client.post(url)
    .header("authorization", AUTHORIZATION)
    .send().await.unwrap()
    .text().await.unwrap();
  let json: Value = serde_json::from_str(&text).unwrap();
  let token = json["guest_token"].as_str().unwrap().to_string();

  token
}

async fn private_api_get(url: Url) -> Value {
  let mut guest_token_mutex = GUEST_TOKEN.lock().await;
  if (*guest_token_mutex).is_empty() {
    *guest_token_mutex = new_guest_token().await;
  }
  let mut guest_token = guest_token_mutex.clone();
  std::mem::drop(guest_token_mutex);

  async fn post_req(url: Url, guest_token: &str) -> Value { 
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append("authorization", AUTHORIZATION.parse().unwrap());
    headers.append("x-guest-token",  guest_token.parse().unwrap());

    let client = reqwest::Client::new();
    let req = client.get(url)
      .headers(headers);
    let text = req.try_clone().unwrap()
      .send().await.unwrap()
      .text().await.unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();

    json
  }

  let mut json = post_req(url.clone(), &guest_token).await;
  // if gave error, re-run the request with a new guest token
  if json["errors"].as_str().is_some() {
    guest_token = new_guest_token().await;
    *GUEST_TOKEN.lock().await = guest_token.clone();
    json = post_req(url.clone(), &guest_token).await;
  }

  json
}

/// fetch the raw json result of a twitter search query
pub async fn query_fetch(query: &str) -> Value {

  let parameters = HashMap::from([
    ("include_profile_interstitial_type", "1"),
    ("include_blocking", "1"),
    ("include_blocked_by", "1"),
    ("include_followed_by", "1"),
    ("include_want_retweets", "1"),
    ("include_mute_edge", "1"),
    ("include_can_dm", "1"),
    ("include_can_media_tag", "1"),
    ("include_ext_has_nft_avatar", "1"),
    ("skip_status", "1"),
    ("cards_platform", "Web-12"),
    ("include_cards", "1"),
    ("include_ext_alt_text", "true"),
    ("include_quote_count", "true"),
    ("include_reply_count", "1"),
    ("tweet_mode", "extended"),
    ("include_entities", "true"),
    ("include_user_entities", "true"),
    ("include_ext_media_color", "true"),
    ("include_ext_media_availability", "true"),
    ("include_ext_sensitive_media_warning", "true"),
    ("include_ext_trusted_friends_metadata", "true"),
    ("send_error_codes", "true"),
    ("simple_quoted_tweet", "true"),
    ("q", query),
    ("tweet_search_mode", "live"),
    ("count", "20"),
    ("query_source", "typed_query"),
    ("pc", "1"),
    ("spelling_corrections", "1"),
    ("ext", "mediaStats,highlightedLabel,hasNftAvatar,voiceInfo,enrichments,superFollowMetadata,unmentionInfo"),
  ]);

  let url = format!("{}{}", PRIVATE_API_BASE, "2/search/adaptive.json?");
  let url = reqwest::Url::parse_with_params(&url, &parameters).unwrap();
  
  let json = private_api_get(url).await
    ["globalObjects"].clone();

  json
}

pub async fn id_fetch(tweet_id: &str, cursor: &str, include_recommended_tweets: bool) -> Option<Value> {
  let with_rux_injections = match include_recommended_tweets {
    true => "true",
    false => "false",
  };
  let mut variables = HashMap::from([
    ("focalTweetId", tweet_id),
    ("with_rux_injections", with_rux_injections), // true = include recommended tweets
    ("includePromotedContent", "false"), // true = include promoted tweets (ads)
    ("withCommunity", "true"), // 🚨🚨🚨🚨🚨 idk???? could be related to promoted content or rux injections
    ("withQuickPromoteEligibilityTweetFields", "false"), // 🚨🚨🚨🚨🚨 idk???? could be related to promoted content or rux injections
    ("withBirdwatchNotes", "false"), // true = add "has_birdwatch_notes" key (val is bool) to tweet_results.result
    ("withSuperFollowsUserFields", "false"), // true = add "super_follow_eligible", "super_followed_by", and "super_following" keys (vals are bool) to user_results.result
    ("withDownvotePerspective", "false"), // 🚨🚨🚨🚨🚨 ACCESS DENIED for true RN, but prob num of downvotes
    ("withReactionsMetadata", "false"), // 🚨🚨🚨🚨🚨 ACCESS DENIED for true RN
    ("withReactionsPerspective", "false"), // 🚨🚨🚨🚨🚨 ACCESS DENIED for true RN
    ("withSuperFollowsTweetFields", "false"), // 🚨🚨🚨🚨🚨 idk????
    ("withVoice", "false"), // 🚨🚨🚨🚨🚨 idk????
    ("withV2Timeline", "true"), // slight change to a small part of the json, but irrelevant for the most part
    ("__fs_responsive_web_like_by_author_enabled", "false"), // true added an ad.. idk why
    ("__fs_dont_mention_me_view_api_enabled", "false"), // true = add "unmention_info" key (val is obj, but seems to always be empty, at least on guest token) to tweet_results.result
    ("__fs_interactive_text_enabled", "true"), // 🚨🚨🚨🚨🚨 idk????
    ("__fs_responsive_web_uc_gql_enabled", "false"), // 🚨🚨🚨🚨🚨 idk????
    ("__fs_responsive_web_edit_tweet_api_enabled", "false"), // 🚨🚨🚨🚨🚨 idk????
  ]);
  // add cursor variable if present
  if cursor != "" {
    variables.insert("cursor", cursor);
  }
  let features = HashMap::from([
    ("standardized_nudges_misinfo", "false"),
  ]);
  let parameters = HashMap::from([
    ("variables", serde_json::to_string(&variables).unwrap()),
    ("features", serde_json::to_string(&features).unwrap()),
  ]);
  let url = format!("{}{}", PRIVATE_API_BASE, "graphql/L1DeQfPt7n3LtTvrBqkJ2g/TweetDetail?");
  let url = reqwest::Url::parse_with_params(&url, &parameters).unwrap();

  let tweets_json = private_api_get(url).await
    .get("data")
    .and_then(|v| v.get("threaded_conversation_with_injections_v2"))
    .and_then(|v| v.get("instructions"))
    .and_then(|v| v.get(0))
    .and_then(|v| {
      // no cursor uses "entries"
      if let Some(v) = v.get("entries") {
        Some(v)
      // cursor uses "moduleItems"
      } else {
        v.get("moduleItems")
      }
  }).unwrap().clone();

  Some(tweets_json)
}