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
  let client = reqwest::Client::builder()
    .gzip(true).deflate(true).brotli(true)
    .build().unwrap();
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
  // check for errors
  if let Some(error) = json.get("errors") {
    let error_code = error[0]["code"].as_i64().unwrap();
    // error code 200: authentication token is expired
    // error code 215: authentication token is missing(/ invalid (?))
    // if authentication error, re-run the request with a new guest token
    if error_code == 200 || error_code == 215 {
      guest_token = new_guest_token().await;
      *GUEST_TOKEN.lock().await = guest_token.clone();
      json = post_req(url.clone(), &guest_token).await;
    // if different error, print the error
    } else {
      println!("twitter get request error: {:?}\n  url: {}", error, url.as_str());
    }
  }

  json
}

/// fetch the raw json result of a twitter search query
pub async fn query_fetch(query: &str) -> Value {

  let parameters = HashMap::from([
    ("include_profile_interstitial_type", "0"), // 1 = include "profile_interstitial_type" attribute in each user object
    ("include_blocking", "0"), // 1 = include "blocking" attribute in each user object
    ("include_blocked_by", "0"), // 1 = include "blocked_by" attribute in each user object
    ("include_followed_by", "0"), // 1 = include "followed_by" attribute in each user object
    ("include_want_retweets", "0"), // 1 = include "want_retweets" attribute in each user object
    ("include_mute_edge", "0"), // 1 = include "muting" attribute in each user object
    ("include_can_dm", "0"), // 1 = include "can_dm" attribute in each user object
    ("include_can_media_tag", "0"), // 🚨🚨🚨 effect unclear
    ("include_ext_has_nft_avatar", "0"), // 1 = include "ext_has_nft_avatar" attribute in each user object
    ("skip_status", "0"), // 🚨🚨🚨 effect unclear
    ("cards_platform", "Web-12"), // 🚨🚨🚨 effect unclear (unsure how to edit "Web-12", but commenting out does nothing)
    ("include_cards", "0"), // 🚨🚨🚨 effect unclear
    ("include_ext_alt_text", "false"), // true = include "ext_alt_text" in tweet objects, and "profile_image_extensions_alt_text" and "profile_banner_extensions_alt_text" in user objects
    ("include_quote_count", "false"), // true = include "quote_count" in tweet objects (num times the tweet has been quote tweeted)
    ("include_reply_count", "0"), // 1 = include "reply_count" in tweet objects (num times the tweet has been replied to)
    ("tweet_mode", "extended"),
    // need this on bc "entities" stores all url conversions, while "extended_entities" 
    // only stores url to do with images and videos
    ("include_entities", "true"), // true = include "entities" object in tweet objects ("entities_extended" is still included if this is false)
    // i think it's fine to have this as false bc i don't think the full detail 
    // included in "entities" will ever be needed for profile info
    ("include_user_entities", "false"), // true = include "entities" object in user objects, but also change url attr of user objects form full url to the shortened url ("entities" must now be used to convert the shortened url to full url)
    ("include_ext_media_color", "false"), // true = include "ext_media_color" object in media objects of "extended_entities" of tweet objects, and "profile_image_extensions_media_color" and "profile_banner_extensions_media_color" of user objects
    ("include_ext_media_availability", "false"), // true = include "ext_media_availability" in media objects of "extended_entities" of tweet objects
    ("include_ext_sensitive_media_warning", "false"), // true = include "ext_sensitive_media_warning" attr in tweet objects, and "profile_image_extensions_sensitive_media_warning" and "profile_banner_extensions_sensitive_media_warning" in user objects
    ("include_ext_trusted_friends_metadata", "false"), // 🚨🚨🚨 effect unclear
    ("send_error_codes", "false"), // 🚨🚨🚨 effect unclear
    ("simple_quoted_tweet", "true"), // true seems to remove the url of the quoted tweet from the quote tweet
    ("q", query),
    ("tweet_search_mode", "live"),
    ("count", "20"),
    ("query_source", "typed_query"),
    ("pc", "0"), // 🚨🚨🚨 effect unclear (what does pc stand for ??? politically correct??)
    ("spelling_corrections", "0"), // 🚨🚨🚨 effect unclear
    // if "ext" = "", "ext" attr is removed from tweet objects, and "extended_entities"
    // media objects from tweet objects, and "profile_image_extensions" and 
    // "ext" from user objects
    // if you instead use the string of a list of names, each name is an 
    // attribute within an "ext" object e.g.
    // `"ext": { "mediaStats": { "r": { "missing": null }, "ttl": -1 }`
    ("ext", "") //"mediaStats,highlightedLabel,hasNftAvatar,voiceInfo,enrichments,superFollowMetadata,unmentionInfo"),
  ]);

  let url = format!("{}{}", PRIVATE_API_BASE, "2/search/adaptive.json?");
  let url = reqwest::Url::parse_with_params(&url, &parameters).unwrap();

  let json = private_api_get(url).await;

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
  ]);                                                 // "L1DeQfPt7n3LtTvrBqkJ2g" is possibly the API version
  let url = format!("{}{}", PRIVATE_API_BASE, "graphql/L1DeQfPt7n3LtTvrBqkJ2g/TweetDetail?");
  let url = reqwest::Url::parse_with_params(&url, &parameters).unwrap();

  let tweets_json = private_api_get(url).await
    .get("data")
    .and_then(|v| v.get("threaded_conversation_with_injections_v2"))
    .and_then(|v| v.get("instructions"))
    .and_then(|v| v.get(0))
    .and_then(|v| {
      // no cursor uses "entries"
      if let Some(e) = v.get("entries") {
        Some(e)
      // cursor uses "moduleItems"
      } else {
        v.get("moduleItems")
      }
  }).unwrap().clone();

  Some(tweets_json)
}

// ----------------------------all (?) graphql APIs----------------------------

// sources -> current page -> scripts -> main.f45ef479.js (https://abs.twimg.com/responsive-web/client-web/main.f45ef479.js)

// 4215: e => {
//     e.exports = {
//         queryId: "zwTrX9CtnMvWlBXjsx95RQ",
//         operationName: "adFreeArticleDomains",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 29643: e => {
//     e.exports = {
//         queryId: "88Bu08U2ddaVVjKmmXjVYg",
//         operationName: "articleNudgeDomains",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 72165: e => {
//     e.exports = {
//         queryId: "_qeAdwr6JrwJ4g8U1FrnTg",
//         operationName: "ArticleTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 9907: e => {
//     e.exports = {
//         queryId: "tvgw5gtcC27iNqQUzy7shg",
//         operationName: "ArticleTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 64332: e => {
//     e.exports = {
//         queryId: "Ha9BKBF0uAz9d4-lz0jnYA",
//         operationName: "AudioSpaceById",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["spaces_2022_h2_clipping", "spaces_2022_h2_spaces_communities", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "responsive_web_graphql_timeline_navigation_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 84924: e => {
//     e.exports = {
//         queryId: "NTq79TuSz6fHj8lQaferJw",
//         operationName: "AudioSpaceSearch",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 51368: e => {
//     e.exports = {
//         queryId: "PFIxTk8owMoZgiMccP0r4g",
//         operationName: "getAltTextPromptPreference",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 99551: e => {
//     e.exports = {
//         queryId: "aQKrduk_DA46XfOQDkcEng",
//         operationName: "updateAltTextPromptPreference",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 69198: e => {
//     e.exports = {
//         queryId: "BwgMOGpOViDS0ri7VUgglg",
//         operationName: "getCaptionsAlwaysDisplayPreference",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 48310: e => {
//     e.exports = {
//         queryId: "uCUQhvZ5sJ9qHinRp6CFlQ",
//         operationName: "updateCaptionsAlwaysDisplayPreference",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 30474: e => {
//     e.exports = {
//         queryId: "QjN8ZdavFDqxUjNn3r9cig",
//         operationName: "AuthenticatedUserTFLists",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 16448: e => {
//     e.exports = {
//         queryId: "pROR-yRiBVsEjJyHt3fvhg",
//         operationName: "BakeryQuery",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 72609: e => {
//     e.exports = {
//         queryId: "3ss48WFwGokBH_gj8t_8aQ",
//         operationName: "BirdwatchAliasSelect",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 74147: e => {
//     e.exports = {
//         queryId: "IF3H8diRqOrKEsiA5Jw0nQ",
//         operationName: "BirdwatchFetchContributorNotesSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 42987: e => {
//     e.exports = {
//         queryId: "TKdL0YFsX4DMOpMKeneLvA",
//         operationName: "BirdwatchCreateAppeal",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 4095: e => {
//     e.exports = {
//         queryId: "6wjp8p4aLDHmA8olWLXK8Q",
//         operationName: "BirdwatchCreateNote",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_graphql_timeline_navigation_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled"]
//         }
//     }
// },
// 24256: e => {
//     e.exports = {
//         queryId: "xWLuzZoATvfEIEGxnsLt7w",
//         operationName: "BirdwatchCreateRating",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 31964: e => {
//     e.exports = {
//         queryId: "IKS_qrShkDyor6Ri1ahd9g",
//         operationName: "BirdwatchDeleteNote",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 36858: e => {
//     e.exports = {
//         queryId: "OpvCOyOoQClUND66zDzrnA",
//         operationName: "BirdwatchDeleteRating",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 18545: e => {
//     e.exports = {
//         queryId: "szoXMke8AZOErso908iglw",
//         operationName: "BirdwatchFetchAliasSelfSelectOptions",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 92661: e => {
//     e.exports = {
//         queryId: "LUEdtkcpBlGktUtms4BvwA",
//         operationName: "BirdwatchFetchAliasSelfSelectStatus",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 68783: e => {
//     e.exports = {
//         queryId: "1AdvbF0AMpC5QNcy2OLeag",
//         operationName: "BirdwatchFetchAuthenticatedUserProfile",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 52751: e => {
//     e.exports = {
//         queryId: "btgGtchypc3D491MJ7XXWA",
//         operationName: "BirdwatchFetchBirdwatchProfile",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 46880: e => {
//     e.exports = {
//         queryId: "dZ0xVYs1BhwfnrajMau1PA",
//         operationName: "BirdwatchFetchGlobalTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 10957: e => {
//     e.exports = {
//         queryId: "OYoxgEiQU3-FqTX8x1AtEw",
//         operationName: "BirdwatchFetchNotes",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_graphql_timeline_navigation_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled"]
//         }
//     }
// },
// 38389: e => {
//     e.exports = {
//         queryId: "gkhw-r_XWNaDSpT9hvB2qw",
//         operationName: "BirdwatchFetchOneNote",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_graphql_timeline_navigation_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled"]
//         }
//     }
// },
// 78111: e => {
//     e.exports = {
//         queryId: "kd54IHJj7owgVY1WqHXhXQ",
//         operationName: "BirdwatchFetchPublicData",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 85949: e => {
//     e.exports = {
//         queryId: "cED9wJy8Nd1kZCCYuIq9zQ",
//         operationName: "BirdwatchProfileAcknowledgeEarnOut",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 61289: e => {
//     e.exports = {
//         queryId: "Rp94IhO-2f4Gqi7MSwBquA",
//         operationName: "BizProfileFetchUser",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 73114: e => {
//     e.exports = {
//         queryId: "rBARf_eozWnJIhQNdvC2GQ",
//         operationName: "BlockedAccountsAll",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 51877: e => {
//     e.exports = {
//         queryId: "LFcHllEwzIgfQGiYngCNNw",
//         operationName: "BlockedAccountsAutoBlock",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 15157: e => {
//     e.exports = {
//         queryId: "m4YfxawdvMes3gDJ2grVAg",
//         operationName: "BlockedAccountsImported",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 98408: e => {
//     e.exports = {
//         queryId: "JnWrqRE9ay3TNi87PGtOgw",
//         operationName: "BookmarkFolderTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 65499: e => {
//     e.exports = {
//         queryId: "N4Ykk6g393xyVY5nmRTkLQ",
//         operationName: "BookmarkFoldersSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 14034: e => {
//     e.exports = {
//         queryId: "4KHZvvNbHNf07bsgnL9gWA",
//         operationName: "bookmarkTweetToFolder",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 76845: e => {
//     e.exports = {
//         queryId: "ea4-_p-ZN9xwhQYOyGpf4w",
//         operationName: "Bookmarks",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["graphql_timeline_v2_bookmark_timeline", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 47546: e => {
//     e.exports = {
//         queryId: "xdgz4z6YU1CNRXD5GY-8vQ",
//         operationName: "CardPreviewByTweetText",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_enhance_cards_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 53757: e => {
//     e.exports = {
//         queryId: "C2dcvh7H69JALtomErxWlA",
//         operationName: "CheckTweetForNudge",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["standardized_nudges_toxicity"]
//         }
//     }
// },
// 24493: e => {
//     e.exports = {
//         queryId: "iy7AGNe9wEON8h59v0DEYg",
//         operationName: "CombinedLists",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 85861: e => {
//     e.exports = {
//         queryId: "dpOOip4Swrgab-mBLx5f_g",
//         operationName: "CommunitiesMainDiscoveryModule",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 95821: e => {
//     e.exports = {
//         queryId: "1u0YeTK6GM6dhvrBSoyZbQ",
//         operationName: "CommunitiesMainPageTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 91509: e => {
//     e.exports = {
//         queryId: "X-68yrVponyHOLxp0fzXJQ",
//         operationName: "CommunitiesMembershipsSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 57460: e => {
//     e.exports = {
//         queryId: "_a4mi0r8adesHaBoK6ehKQ",
//         operationName: "CommunitiesMembershipsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 43160: e => {
//     e.exports = {
//         queryId: "sRNV-VmshHvuuzdUb7u87Q",
//         operationName: "CommunityAboutTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 27986: e => {
//     e.exports = {
//         queryId: "uUUni5GBsZ7dsL3OAfxqZg",
//         operationName: "CreateCommunity",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 61054: e => {
//     e.exports = {
//         queryId: "MUys_jLrkVDM7fVMlJhsEQ",
//         operationName: "CommunityCreateRule",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 66249: e => {
//     e.exports = {
//         queryId: "SIHV2OzCk8e9naIfwhalVQ",
//         operationName: "CommunityDiscoveryTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 12235: e => {
//     e.exports = {
//         queryId: "gI0aWiPJYl2Q1PMazHhmOg",
//         operationName: "CommunityEditBannerMedia",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 12773: e => {
//     e.exports = {
//         queryId: "ldBvUb6dbcOdGXP3vf5Y3g",
//         operationName: "CommunityEditName",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 3211: e => {
//     e.exports = {
//         queryId: "0Uw80o1gh1pQ2RJx8u0K_A",
//         operationName: "CommunityEditPurpose",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 11382: e => {
//     e.exports = {
//         queryId: "oAA9Gyc5UhAZP3-_aUo88g",
//         operationName: "CommunityEditRule",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 4389: e => {
//     e.exports = {
//         queryId: "DaPQS4RoYrILg-QX-kwIyw",
//         operationName: "CommunityEditTheme",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 9797: e => {
//     e.exports = {
//         queryId: "GS_YnJexJjRMdr8wXyajPA",
//         operationName: "CommunityByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 13345: e => {
//     e.exports = {
//         queryId: "KynZkoypT29vbOd143Ps8w",
//         operationName: "CommunityHashtagsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 13003: e => {
//     e.exports = {
//         queryId: "u4PgNqze4A2T17GbySYraw",
//         operationName: "JoinCommunity",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 4093: e => {
//     e.exports = {
//         queryId: "DWBEg-QgQc0QNKdBNZ5b1A",
//         operationName: "LeaveCommunity",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 84478: e => {
//     e.exports = {
//         queryId: "npEFiYc3sCX-YDHZ1B69nw",
//         operationName: "CommunityMemberRelationshipTypeahead",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled"]
//         }
//     }
// },
// 87192: e => {
//     e.exports = {
//         queryId: "a7OcKHSVGRK0GYOa11caBQ",
//         operationName: "CommunityModerationKeepTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 91250: e => {
//     e.exports = {
//         queryId: "d-hbNWQ8GpfTGgEqbDNaqQ",
//         operationName: "CommunityModerationTweetCasesSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 36058: e => {
//     e.exports = {
//         queryId: "VWrnXHdLZCY3dFew72V86g",
//         operationName: "CommunityRemoveBannerMedia",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 44565: e => {
//     e.exports = {
//         queryId: "oLsw1u3HutX9Tax-dr88Ow",
//         operationName: "CommunityRemoveRule",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 16786: e => {
//     e.exports = {
//         queryId: "jIq-IIeWvp5AKNRBFTSiaA",
//         operationName: "CommunityReorderRules",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 4934: e => {
//     e.exports = {
//         queryId: "kloa05ND5gN5M0e2Jey9-Q",
//         operationName: "RequestToJoinCommunity",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 26130: e => {
//     e.exports = {
//         queryId: "z4QDKtA8JDqjaMhkW2DEzA",
//         operationName: "CommunityTweetsRankedTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 74777: e => {
//     e.exports = {
//         queryId: "WkELlyxUKipDSLH7nqnMaw",
//         operationName: "CommunityTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 86153: e => {
//     e.exports = {
//         queryId: "5SPJ9B2kOYlaqwfdQ7htZw",
//         operationName: "CommunityUpdateRole",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 4091: e => {
//     e.exports = {
//         queryId: "el2xSrgrXBxyWvbz193qaA",
//         operationName: "CommunityUserInvite",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled"]
//         }
//     }
// },
// 76776: e => {
//     e.exports = {
//         queryId: "C9CUCfAvlaMEhJBmM2hZkg",
//         operationName: "CommunityUserRelationshipTypeahead",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled"]
//         }
//     }
// },
// 42945: e => {
//     e.exports = {
//         queryId: "rLuUGLOJKAQyqaALd-IREg",
//         operationName: "ConnectTabTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 21341: e => {
//     e.exports = {
//         queryId: "N6oDUH0bX-7w9Cj4Wd6zXA",
//         operationName: "ContentControlToolDisable",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 18195: e => {
//     e.exports = {
//         queryId: "4dMgLjwMMTqp33sMQ6IrJA",
//         operationName: "ContentControlToolEnable",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 99886: e => {
//     e.exports = {
//         queryId: "nMDl1pqwn1QotVzqPAjjOw",
//         operationName: "ContentControlToolFetchAll",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 76092: e => {
//     e.exports = {
//         queryId: "kOKEPqQB769u1TI5FJt-tw",
//         operationName: "ContentControlToolFetchAllUserEnabled",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 7357: e => {
//     e.exports = {
//         queryId: "JD2EtsNFZ3uvvTP_lfn9Bg",
//         operationName: "ContentControlToolFetchOne",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 8378: e => {
//     e.exports = {
//         queryId: "hb1elGcj6769uT8qVYqtjw",
//         operationName: "ConversationControlChange",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 2536: e => {
//     e.exports = {
//         queryId: "OoMO_aSZ1ZXjegeamF9QmA",
//         operationName: "ConversationControlDelete",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 12875: e => {
//     e.exports = {
//         queryId: "2njnYoE69O2jdUM7KMEnDw",
//         operationName: "ConvertRitoSuggestedActions",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 78152: e => {
//     e.exports = {
//         queryId: "aoDbu3RHznuiSkQ9aNM67Q",
//         operationName: "CreateBookmark",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 13673: e => {
//     e.exports = {
//         queryId: "6Xxqpq8TM_CREYiuof_h5w",
//         operationName: "createBookmarkFolder",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 49390: e => {
//     e.exports = {
//         queryId: "2LHXrd1uYeaMWhciZgPZFw",
//         operationName: "CreateCustomerPortalSession",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 51052: e => {
//     e.exports = {
//         queryId: "cH9HZWz_EW9gnswvA4ZRiQ",
//         operationName: "CreateDraftTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 40558: e => {
//     e.exports = {
//         queryId: "ojPdsZsimiJrUGLR1sjUtA",
//         operationName: "CreateRetweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 85141: e => {
//     e.exports = {
//         queryId: "LCVzRQGxOaGnOnYH01NQXg",
//         operationName: "CreateScheduledTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 21037: e => {
//     e.exports = {
//         queryId: "2tP8XUYeLHKjq5RHvuvpZw",
//         operationName: "CreateTrustedFriendsList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 53934: e => {
//     e.exports = {
//         queryId: "znCBT5T-VuJFVKmKvj2RVQ",
//         operationName: "CreateTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "responsive_web_graphql_timeline_navigation_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 45373: e => {
//     e.exports = {
//         queryId: "Eo65jl-gww30avDgrXvhUA",
//         operationName: "CreateTweetDownvote",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 99090: e => {
//     e.exports = {
//         queryId: "D7M6X3h4-mJE8UB1Ap3_dQ",
//         operationName: "CreateTweetReaction",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 84605: e => {
//     e.exports = {
//         queryId: "xF6sXnKJfS2AOylzxRjf6A",
//         operationName: "DataSaverMode",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 77146: e => {
//     e.exports = {
//         queryId: "H03etWvZGz41YASxAU2YPg",
//         operationName: "WriteDataSaverPreferences",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 92929: e => {
//     e.exports = {
//         queryId: "QEMLEzEMzoPNbeauKCCLbg",
//         operationName: "SetDefault",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 93508: e => {
//     e.exports = {
//         queryId: "skiACZKC1GDYli-M8RzEPQ",
//         operationName: "BookmarksAllDelete",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 44577: e => {
//     e.exports = {
//         queryId: "Wlmlj2-xzyS1GN3a6cj-mQ",
//         operationName: "DeleteBookmark",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 13647: e => {
//     e.exports = {
//         queryId: "2UTTsO-6zs93XqlEUZPsSg",
//         operationName: "DeleteBookmarkFolder",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 7690: e => {
//     e.exports = {
//         queryId: "bkh9G3FGgTldS9iTKWWYYw",
//         operationName: "DeleteDraftTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 27913: e => {
//     e.exports = {
//         queryId: "VaaLGwK5KNLoc7wsOmp4uw",
//         operationName: "DeletePaymentMethod",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 60445: e => {
//     e.exports = {
//         queryId: "iQtK4dl5hBmXewYZuEOKVw",
//         operationName: "DeleteRetweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 97939: e => {
//     e.exports = {
//         queryId: "CTOVqej0JBXAZSwkp1US0g",
//         operationName: "DeleteScheduledTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 9578: e => {
//     e.exports = {
//         queryId: "VaenaVgh5q5ih7kvyVjgtg",
//         operationName: "DeleteTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 93764: e => {
//     e.exports = {
//         queryId: "VNEvEGXaUAMfiExP8Tbezw",
//         operationName: "DeleteTweetDownvote",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 38150: e => {
//     e.exports = {
//         queryId: "GKwK0Rj4EdkfwdHQMZTpuw",
//         operationName: "DeleteTweetReaction",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 91939: e => {
//     e.exports = {
//         queryId: "_ckHEj05gan2VfNHG6thBA",
//         operationName: "DisableUserAccountLabel",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 69685: e => {
//     e.exports = {
//         queryId: "g2m0pAOamawNtVIfjXNMJg",
//         operationName: "DisableVerifiedPhoneLabel",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 45493: e => {
//     e.exports = {
//         queryId: "jYvwa61cv3NwNP24iUru6g",
//         operationName: "DismissRitoSuggestedAction",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 88823: e => {
//     e.exports = {
//         queryId: "ZjdL4uZkhRXCPKGE5CRtEw",
//         operationName: "DmAllSearchSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 47448: e => {
//     e.exports = {
//         queryId: "5zpY1dCR-8NyxQJS_CFJoQ",
//         operationName: "DmGroupSearchSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 87348: e => {
//     e.exports = {
//         queryId: "y3uKc4Gmwq0nzuubHk1ksg",
//         operationName: "DmMutedTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 94958: e => {
//     e.exports = {
//         queryId: "of_N6O33zfyD4qsFJMYFxA",
//         operationName: "DmNsfwMediaFilterUpdate",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 64987: e => {
//     e.exports = {
//         queryId: "xYSm8m5kJnzm_gFCn5GH-w",
//         operationName: "DmPeopleSearchSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 18438: e => {
//     e.exports = {
//         queryId: "a6kPp1cS1Dgbsjhapz1PNw",
//         operationName: "EditBookmarkFolder",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 20074: e => {
//     e.exports = {
//         queryId: "JIeXE-I6BZXHfxsgOkyHYQ",
//         operationName: "EditDraftTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 63789: e => {
//     e.exports = {
//         queryId: "_mHkQ5LHpRRjSXKOcG6eZw",
//         operationName: "EditScheduledTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 61537: e => {
//     e.exports = {
//         queryId: "2qKKYFQift8p5-J1k6kqxQ",
//         operationName: "WriteEmailNotificationSettings",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 72298: e => {
//     e.exports = {
//         queryId: "BqIHKmwZKtiUBPi07jKctg",
//         operationName: "EnableLoggedOutWebNotifications",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 62152: e => {
//     e.exports = {
//         queryId: "C3RJFfMsb_KcEytpKmRRkw",
//         operationName: "EnableVerifiedPhoneLabel",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 69875: e => {
//     e.exports = {
//         queryId: "lI07N6Otwv1PhnEgXILM7A",
//         operationName: "FavoriteTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 68110: e => {
//     e.exports = {
//         queryId: "vHwDErHAReTBqHY_N9aHaA",
//         operationName: "Favoriters",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 1819: e => {
//     e.exports = {
//         queryId: "-btar_vkBwWA7s3YWfp_9g",
//         operationName: "FeatureSettingsUpdate",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 63922: e => {
//     e.exports = {
//         queryId: "_gXC5CopoM8fIgawvyGpIg",
//         operationName: "Followers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 13004: e => {
//     e.exports = {
//         queryId: "0hh4SWxF5ZLArQEJNig7kg",
//         operationName: "FollowersYouKnow",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 70395: e => {
//     e.exports = {
//         queryId: "9rGM7YNDYuiqd0Cb0ZwLJw",
//         operationName: "Following",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 84318: e => {
//     e.exports = {
//         queryId: "WGCFbp2BqGlP3NGt2DJ8zQ",
//         operationName: "ForYouExplore",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 84559: e => {
//     e.exports = {
//         queryId: "00Sr8WO2kvcZ2hrQfH0w7g",
//         operationName: "GenericTimelineById",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 12657: e => {
//     e.exports = {
//         queryId: "2V2W3HIBuMW83vEMtfo_Rg",
//         operationName: "GraphQLError",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 56653: e => {
//     e.exports = {
//         queryId: "bseRa9LB6u_tOAwXQXtbOg",
//         operationName: "HomeLatestTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 58659: e => {
//     e.exports = {
//         queryId: "3Jlcsc8QPuEkeos09P2V2g",
//         operationName: "HomeTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 57765: e => {
//     e.exports = {
//         queryId: "36pMOwzEAOigRVwjHcP1ZA",
//         operationName: "ImmersiveMedia",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 52318: e => {
//     e.exports = {
//         queryId: "lr2pk7rKqCqLSqWRGRaW5Q",
//         operationName: "Likes",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 4033: e => {
//     e.exports = {
//         queryId: "oD7xZZJ4l7RJqD3vgRMWjA",
//         operationName: "ListAddMember",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 3596: e => {
//     e.exports = {
//         queryId: "_7mC01JCOYv2y-oDkNq8jg",
//         operationName: "DeleteListBanner",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 28983: e => {
//     e.exports = {
//         queryId: "mSN9U0iUa2z4fFg0JTO2qQ",
//         operationName: "EditListBanner",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 59852: e => {
//     e.exports = {
//         queryId: "Gz6SODtBmiJiPUM6EhDZug",
//         operationName: "ListBySlug",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 31892: e => {
//     e.exports = {
//         queryId: "33U6lIy9S_CjNSgYXKDEng",
//         operationName: "CreateList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 12253: e => {
//     e.exports = {
//         queryId: "F6hNGYHhwU1psHm-Dnh2AA",
//         operationName: "ListCreationRecommendedUsers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 83934: e => {
//     e.exports = {
//         queryId: "UnN9Th1BDbeLjpgjGSpL3Q",
//         operationName: "DeleteList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 24533: e => {
//     e.exports = {
//         queryId: "ZkqIq_xRhiUme0PBJNpRtg",
//         operationName: "FetchDraftTweets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 27706: e => {
//     e.exports = {
//         queryId: "Hpy_C2fuR_6NTGVWEhmffw",
//         operationName: "ListEditRecommendedUsers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 62106: e => {
//     e.exports = {
//         queryId: "hpd0JOlf0m0oiqvSmGkq1w",
//         operationName: "ListLatestTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 79005: e => {
//     e.exports = {
//         queryId: "6i82-1wRWNkzReI8o3yfUQ",
//         operationName: "ListMembers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 20744: e => {
//     e.exports = {
//         queryId: "hJYxPjx4OxE4fiHw-j4YtA",
//         operationName: "ListMemberships",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 63222: e => {
//     e.exports = {
//         queryId: "ZYyanJsskNUcltu9bliMLA",
//         operationName: "MuteList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 1655: e => {
//     e.exports = {
//         queryId: "W1GyVNu0kB4nA0AhLpuWQQ",
//         operationName: "ListOwnerships",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 42177: e => {
//     e.exports = {
//         queryId: "mECa8rxMnKsCEpt3Ryasbg",
//         operationName: "ListPinOne",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 51947: e => {
//     e.exports = {
//         queryId: "REGT5XU2K_4x8KQ-rjsIVw",
//         operationName: "ListPins",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 81141: e => {
//     e.exports = {
//         queryId: "wwdBYgScze0_Jnan79jEUw",
//         operationName: "ListProductSubscriptions",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 87587: e => {
//     e.exports = {
//         queryId: "3Rnx7ebPrd5yx3EIlBqeOQ",
//         operationName: "ListByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 45946: e => {
//     e.exports = {
//         queryId: "EI3jYmvI-xzDS1LCyeW6jA",
//         operationName: "ListRankedTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 34246: e => {
//     e.exports = {
//         queryId: "-UD5zcmE1K-ByMCpsMQXNA",
//         operationName: "ListRemoveMember",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 34622: e => {
//     e.exports = {
//         queryId: "_71kktdGu233FoLHbYsGWQ",
//         operationName: "ListSubscribe",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 31297: e => {
//     e.exports = {
//         queryId: "tzTyyV3TeaAc54G9lvsh-Q",
//         operationName: "ListSubscribers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 67778: e => {
//     e.exports = {
//         queryId: "pMZrHRNsmEkXgbn3tOyr7Q",
//         operationName: "UnmuteList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 80291: e => {
//     e.exports = {
//         queryId: "ITSJj_mKU9Wg3IbNzfzp_w",
//         operationName: "ListUnpinOne",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 24242: e => {
//     e.exports = {
//         queryId: "gEw93c2Glk3vwM1KnrV9AQ",
//         operationName: "ListUnsubscribe",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 83603: e => {
//     e.exports = {
//         queryId: "Am63m8FeQRQ8752q0nH01A",
//         operationName: "UpdateList",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 27229: e => {
//     e.exports = {
//         queryId: "PL2pjufsPbrRhYaKIE4PGw",
//         operationName: "ListsDiscovery",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 87903: e => {
//     e.exports = {
//         queryId: "baWPOSy2E5oWX7NAWbJVjw",
//         operationName: "ListsManagementPageTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 16914: e => {
//     e.exports = {
//         queryId: "6SAF4mccyY_5xCS2l2PIiw",
//         operationName: "ListsPinMany",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 73254: e => {
//     e.exports = {
//         queryId: "-lnNX56S2YrZYrLzbccFAQ",
//         operationName: "LiveCommerceItemsSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 25912: e => {
//     e.exports = {
//         queryId: "pjFnHGVqCjTcZol0xcBJjw",
//         operationName: "ModerateTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 53846: e => {
//     e.exports = {
//         queryId: "c9IdrvgCZw7oxPZFPBpyrg",
//         operationName: "ModeratedTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 44328: e => {
//     e.exports = {
//         queryId: "ULTlax3qxeRRfbT8bya1TQ",
//         operationName: "MutedAccounts",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 73277: e => {
//     e.exports = {
//         queryId: "xQUzNxuokiu-ApybwO77ow",
//         operationName: "NoteworthyAccountsPage",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 35422: e => {
//     e.exports = {
//         queryId: "mPF_G9okpbZuLcD6mN8K9g",
//         operationName: "PaymentMethods",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 27366: e => {
//     e.exports = {
//         queryId: "GA2_1uKP9b_GyR4MVAQXAw",
//         operationName: "PinReply",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 93137: e => {
//     e.exports = {
//         queryId: "iRe6ig5OV1EzOtldNIuGDQ",
//         operationName: "UnpinReply",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 20784: e => {
//     e.exports = {
//         queryId: "5kUWP8C1hcd6omvg6HXXTQ",
//         operationName: "ProfileUserPhoneState",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 82100: e => {
//     e.exports = {
//         queryId: "IjQ-egg0uPkY11NyPMfRMQ",
//         operationName: "PutClientEducationFlag",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 74954: e => {
//     e.exports = {
//         queryId: "a8KxGfFQAmm3WxqemuqSRA",
//         operationName: "AdAccounts",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 66203: e => {
//     e.exports = {
//         queryId: "1LYVUabJBYkPlUAWRabB3g",
//         operationName: "AudienceEstimate",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 70648: e => {
//     e.exports = {
//         queryId: "mbK3oSQotwcJXyQIBE3uYw",
//         operationName: "Budgets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 25014: e => {
//     e.exports = {
//         queryId: "R1h43jnAl2bsDoUkgZb7NQ",
//         operationName: "Coupons",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 35697: e => {
//     e.exports = {
//         queryId: "O2cSZOui7LQfmVnmawyXpw",
//         operationName: "CreateQuickPromotion",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 46745: e => {
//     e.exports = {
//         queryId: "LtpCXh66W-uXh7u7XSRA8Q",
//         operationName: "QuickPromoteEligibility",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 88832: e => {
//     e.exports = {
//         queryId: "SOyGmNGaEXcvk15s5bqDrA",
//         operationName: "EnrollCoupon",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 16881: e => {
//     e.exports = {
//         queryId: "QpNfg0kpPRfjROQ_9eOLXA",
//         operationName: "RemoveFollower",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 90827: e => {
//     e.exports = {
//         queryId: "2Qbj9XZvtUvyJB4gFwWfaA",
//         operationName: "RemoveTweetFromBookmarkFolder",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 36852: e => {
//     e.exports = {
//         queryId: "I09N_p5WxiCz5mqt034wSA",
//         operationName: "Retweeters",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 10337: e => {
//     e.exports = {
//         queryId: "NFARV9iE-FIK7FOsO_d23Q",
//         operationName: "RevueAccountByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 77828: e => {
//     e.exports = {
//         queryId: "St0cxU6IRuItc7f0Ps-a9g",
//         operationName: "RitoActionedTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 75169: e => {
//     e.exports = {
//         queryId: "21YhSM0hck0RKIXme6evXQ",
//         operationName: "RitoFlaggedAccountsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 58912: e => {
//     e.exports = {
//         queryId: "EWbD2e8CZCVFE_2uwBgKnQ",
//         operationName: "RitoFlaggedTweetsTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 75339: e => {
//     e.exports = {
//         queryId: "GnQKeEdL1LyeK3dTQCS1yw",
//         operationName: "RitoSuggestedActionsFacePile",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 79384: e => {
//     e.exports = {
//         queryId: "AhxTX0lkbIos4WG53xwzSA",
//         operationName: "GetSafetyModeSettings",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 72710: e => {
//     e.exports = {
//         queryId: "qSJIPIpf4gA7Wn21bT3D4w",
//         operationName: "SetSafetyModeSettings",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 15299: e => {
//     e.exports = {
//         queryId: "ITtjAzvlZni2wWXwf295Qg",
//         operationName: "FetchScheduledTweets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 95876: e => {
//     e.exports = {
//         queryId: "5h0kNbk3ii97rmfY6CdgAA",
//         operationName: "SharingAudiospacesListeningDataWithFollowersUpdate",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 33627: e => {
//     e.exports = {
//         queryId: "yUpmLojxsGPk0Sh3JHfEcQ",
//         operationName: "SubcribeToRevueNewsletter",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 24135: e => {
//     e.exports = {
//         queryId: "Sxn4YOlaAwEKjnjWV0h7Mw",
//         operationName: "SubscribeToScheduledSpace",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 49228: e => {
//     e.exports = {
//         queryId: "z3EuIg-hYaGwJN6Ktaw3-Q",
//         operationName: "SubscriptionCheckoutUrl",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 9105: e => {
//     e.exports = {
//         queryId: "KBV74cHcInP4YhJqCA4isQ",
//         operationName: "SubscriptionProductDetails",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 73402: e => {
//     e.exports = {
//         queryId: "Me2CVcAXxvK2WMr-Nh_Qqg",
//         operationName: "SubscriptionProductFeaturesFetch",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 51583: e => {
//     e.exports = {
//         queryId: "DmGBcxuI4lWd3z_mV_2M6Q",
//         operationName: "SuperFollowers",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 43147: e => {
//     e.exports = {
//         queryId: "vfVbgvTPTQ-dF_PQ5lD1WQ",
//         operationName: "timelinesFeedback",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 11405: e => {
//     e.exports = {
//         queryId: "ElqSLWFmsPL4NlZI5e1Grg",
//         operationName: "TopicFollow",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 10522: e => {
//     e.exports = {
//         queryId: "LQAUgm0DiMTFcK98fd0KCQ",
//         operationName: "TopicLandingPage",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 15065: e => {
//     e.exports = {
//         queryId: "cPCFdDAaqRjlMRYInZzoDA",
//         operationName: "TopicNotInterested",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 7968: e => {
//     e.exports = {
//         queryId: "4OUZZOonV2h60I0wdlQb_w",
//         operationName: "TopicByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 68903: e => {
//     e.exports = {
//         queryId: "4emZpe0r583TRJwnuYnkBA",
//         operationName: "TopicTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 81138: e => {
//     e.exports = {
//         queryId: "92aqkmAM0LY20tCAPdOtvw",
//         operationName: "TopicToFollowSidebar",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 2796: e => {
//     e.exports = {
//         queryId: "4tVnt6FoSxaX8L-mDDJo4Q",
//         operationName: "TopicUndoNotInterested",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 47156: e => {
//     e.exports = {
//         queryId: "srwjU6JM_ZKTj_QMfUGNcw",
//         operationName: "TopicUnfollow",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 86069: e => {
//     e.exports = {
//         queryId: "Lt9WPkNBUP-LtG_OPW9FkA",
//         operationName: "TopicsManagementPage",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 66206: e => {
//     e.exports = {
//         queryId: "wXuiyRAuwYlz8AO2gpbuDg",
//         operationName: "TopicsPickerPage",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 25455: e => {
//     e.exports = {
//         queryId: "xE9YgVic_l4XyZDSwVb1ow",
//         operationName: "TopicsPickerPageById",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 55895: e => {
//     e.exports = {
//         queryId: "wfVaMHyeA2Y0cKn9RziHmA",
//         operationName: "TrustedFriendsTypeahead",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled"]
//         }
//     }
// },
// 47278: e => {
//     e.exports = {
//         queryId: "BoHLKeBvibdYDiJON1oqTg",
//         operationName: "TweetDetail",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 20431: e => {
//     e.exports = {
//         queryId: "HI4zM7ZsAfEUxCP7ODlBkA",
//         operationName: "TweetEditHistory",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "responsive_web_enhance_cards_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled"]
//         }
//     }
// },
// 61710: e => {
//     e.exports = {
//         queryId: "yNUmEuKlmggi-HFK87MbIw",
//         operationName: "GetTweetReactionTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 24586: e => {
//     e.exports = {
//         queryId: "RKTEerre_yxxOJplwwXdpQ",
//         operationName: "TweetResultByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 32399: e => {
//     e.exports = {
//         queryId: "EvbTkPDT-xQCfupPu0rWMA",
//         operationName: "TweetStats",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["profile_foundations_tweet_stats_enabled", "profile_foundations_tweet_stats_tweet_frequency"]
//         }
//     }
// },
// 43746: e => {
//     e.exports = {
//         queryId: "duJJegN1edPOcPOW-jKp6g",
//         operationName: "TwitterArticleCreate",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 96430: e => {
//     e.exports = {
//         queryId: "6st-stMDc7KBqLT8KvWhHg",
//         operationName: "TwitterArticleDelete",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 18424: e => {
//     e.exports = {
//         queryId: "uLmpbyRjm8BYhMTWr5-vcw",
//         operationName: "TwitterArticleByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 91661: e => {
//     e.exports = {
//         queryId: "-hpqsWfmxY9YccXcLf15FA",
//         operationName: "TwitterArticleUpdateCoverImage",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 4588: e => {
//     e.exports = {
//         queryId: "FhMX48N0zFpop3NdOHyezQ",
//         operationName: "TwitterArticleUpdateData",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 9181: e => {
//     e.exports = {
//         queryId: "YrFGxKBGKAGxHqhqHIHLGw",
//         operationName: "TwitterArticleUpdateMedia",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 27189: e => {
//     e.exports = {
//         queryId: "BhQCHCV15_tURwY5-X41HA",
//         operationName: "TwitterArticleUpdateTitle",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 45776: e => {
//     e.exports = {
//         queryId: "n8UG7F56szDxZhwryRZr0Q",
//         operationName: "TwitterArticleUpdateVisibility",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 65523: e => {
//     e.exports = {
//         queryId: "_WccxtEQTj3QAVTc2F_u7w",
//         operationName: "TwitterArticlesSlice",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_article_data_v2_enabled", "responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 84461: e => {
//     e.exports = {
//         queryId: "ZYKSe-w7KEslx3JhSIk5LA",
//         operationName: "UnfavoriteTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 63455: e => {
//     e.exports = {
//         queryId: "xVW9j3OqoBRY9d6_2OONEg",
//         operationName: "UnmentionUserFromConversation",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 17439: e => {
//     e.exports = {
//         queryId: "pVSyu6PA57TLvIE4nN2tsA",
//         operationName: "UnmoderateTweet",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 93957: e => {
//     e.exports = {
//         queryId: "Zevhh76Msw574ZSs2NQHGQ",
//         operationName: "UnsubscribeFromScheduledSpace",
//         operationType: "mutation",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 9791: e => {
//     e.exports = {
//         queryId: "EMgItvgUsx44IFgSaNKzBw",
//         operationName: "UrtFixtures",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 25886: e => {
//     e.exports = {
//         queryId: "G4NsXlMBD0izBYpilEDvQQ",
//         operationName: "UserAboutTimeline",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 95378: e => {
//     e.exports = {
//         queryId: "rD5gLxVmMvtdtYU1UHWlFQ",
//         operationName: "UserAccountLabel",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 88441: e => {
//     e.exports = {
//         queryId: "Qs44y3K0SXxItjNi6mUFQA",
//         operationName: "UserByRestId",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 44538: e => {
//     e.exports = {
//         queryId: "ptQPCD7NrFS_TW71Lq07nw",
//         operationName: "UserByScreenName",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 98453: e => {
//     e.exports = {
//         queryId: "lFi3xnx0auUUnyG4YwpCNw",
//         operationName: "GetUserClaims",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 83038: e => {
//     e.exports = {
//         queryId: "pPv9g3uSVHOxOCMpx0Gtug",
//         operationName: "UserMedia",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 46977: e => {
//     e.exports = {
//         queryId: "56eGxRLB5xReLdWB0PNaig",
//         operationName: "UserPromotableTweets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 27141: e => {
//     e.exports = {
//         queryId: "vJ-XatpmQSG8bDch8-t9Jw",
//         operationName: "UserSessionsList",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 37696: e => {
//     e.exports = {
//         queryId: "osIED89Xk0_NKFqgSWpgJg",
//         operationName: "UserSuperFollowTweets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 96520: e => {
//     e.exports = {
//         queryId: "25oeBocoJ0NLTbSBegxleg",
//         operationName: "UserTweets",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 72975: e => {
//     e.exports = {
//         queryId: "s0hG9oAmWEYVBqOLJP-TBQ",
//         operationName: "UserTweetsAndReplies",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },
// 82079: e => {
//     e.exports = {
//         queryId: "_MrlIB_y3BWSLB-XeU9XVA",
//         operationName: "UsersByRestIds",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 13736: e => {
//     e.exports = {
//         queryId: "AkfLpq1RURPtDOcd56qyCg",
//         operationName: "UsersVerifiedAvatars",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled"]
//         }
//     }
// },
// 29544: e => {
//     e.exports = {
//         queryId: "JpjlNgn4sLGvS6tgpTzYBg",
//         operationName: "ViewerEmailSettings",
//         operationType: "query",
//         metadata: {
//             featureSwitches: []
//         }
//     }
// },
// 95674: e => {
//     e.exports = {
//         queryId: "iugWi6fZBxE7Qzt_5PiIYw",
//         operationName: "Viewer",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 22227: e => {
//     e.exports = {
//         queryId: "fY0HP5NgWOZRh7BWZs8tjw",
//         operationName: "ViewerTeams",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled"]
//         }
//     }
// },
// 14246: e => {
//     e.exports = {
//         queryId: "G29lL2Yy-6ha1BSJbhfp0Q",
//         operationName: "ViewingOtherUsersTopicsPage",
//         operationType: "query",
//         metadata: {
//             featureSwitches: ["responsive_web_twitter_blue_verified_badge_is_enabled", "verified_phone_label_enabled", "responsive_web_graphql_timeline_navigation_enabled", "unified_cards_ad_metadata_container_dynamic_card_content_query_enabled", "tweetypie_unmention_optimization_enabled", "responsive_web_uc_gql_enabled", "vibe_api_enabled", "responsive_web_edit_tweet_api_enabled", "graphql_is_translatable_rweb_tweet_is_translatable_enabled", "standardized_nudges_misinfo", "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled", "interactive_text_enabled", "responsive_web_text_conversations_enabled", "responsive_web_enhance_cards_enabled"]
//         }
//     }
// },