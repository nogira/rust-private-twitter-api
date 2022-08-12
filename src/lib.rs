mod types;
use types::{QueryTweet, TweetMedia, TweetURLs, Quote};

use std::{collections::HashMap};
// use std::sync::Mutex;
use parking_lot::Mutex;
use serde_json::Value;
use regex::Regex;

const AUTHORIZATION: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
const PRIVATE_API_BASE: &str = "https://twitter.com/i/api/";

static GUEST_TOKEN: Mutex<String> = Mutex::new(String::new());

/// get "x-guest-token" for subsequent requests
pub fn new_guest_token() -> String {
  let url = "https://api.twitter.com/1.1/guest/activate.json";
  let client = reqwest::blocking::Client::new();
  let token = client.post(url)
    .header("authorization", AUTHORIZATION)
    .send().unwrap()
    .json::<Value>().unwrap()
    ["guest_token"].as_str().unwrap().to_string();

  token
}

/// fetch the raw json result of a twitter search query
pub fn query_fetch(query: &str) -> Value {

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
  
  let mut guest_token = GUEST_TOKEN.lock();
  if (*guest_token).is_empty() {
    *guest_token = new_guest_token();
  }

  let mut headers = reqwest::header::HeaderMap::new();
  headers.append("authorization", AUTHORIZATION.parse().unwrap());
  headers.append("x-guest-token",  (*guest_token).parse().unwrap());
  let client = reqwest::blocking::Client::new();
  let req = client.get(url)
    .headers(headers);

  let mut json: Value = req.try_clone().unwrap()
    .send().unwrap()
    .json::<Value>().unwrap();
  // if gave error, re-run the request with a new guest token
  if json["errors"].as_str().is_some() {
    *guest_token = new_guest_token();
    json = req.try_clone().unwrap()
      .send().unwrap()
      .json::<Value>().unwrap();
  }

  json["globalObjects"].clone()
}

pub fn query_to_tweets(query: &str) -> Vec<QueryTweet> {

  let fetch_json = query_fetch(query);

  // data is separated into users and tweets, so to attach username to tweet, 
  // need to get user info first

  /* -------------------------------- users -------------------------------- */
  let users_json = fetch_json["users"].as_object().unwrap();
  let mut user_id_to_name_map: HashMap<&str, &str> = HashMap::new();
  for (_, user_json) in users_json {
    let id = user_json["id_str"].as_str().unwrap();
    let name = user_json["screen_name"].as_str().unwrap();
    user_id_to_name_map.insert(id, name);
  }

  let mut parsed_tweets: Vec<QueryTweet> = Vec::new();

  let tweets_json = fetch_json["tweets"].as_object().unwrap();

  for (_, tweet_json) in tweets_json {
    let id = tweet_json["id_str"].as_str().unwrap().to_string();

    let user = user_id_to_name_map[tweet_json["user_id_str"].as_str().unwrap()].to_string();

    let text = tweet_json["full_text"].as_str().unwrap().to_string();

    let media = {
      if let Some(media_json) = tweet_json["extended_entities"]["media"].as_array() {
        let mut media: Vec<TweetMedia> = Vec::new();

        for item in media_json {
          let shortened_img_url = item["url"].as_str().unwrap().to_string();
          let full_img_url = item["media_url_https"].as_str().unwrap().to_string();
          let kind = item["type"].as_str().unwrap().to_string();

          let mut video_url: Option<String> = None;
          if kind == "video" {
            // sort array by bitrate so that the highest bitrate variant is first in 
            // the array. the .m3u8` variant doesn't have a bitrate property, so must 
            // use `?? -1` to push it to the end of the array
            let variants = item["video_info"]["variants"].as_array().unwrap();
            let mut highest_bitrate = 0;
            let mut highest_bitrate_mp4_url = "";
            for variant in variants {
              // need default val bc one of the variants never has a bitrate val
              let bitrate = variant["bitrate"].as_i64().unwrap_or(0);
              if bitrate > highest_bitrate {
                highest_bitrate = bitrate;
                highest_bitrate_mp4_url = variant["url"].as_str().unwrap();
              }
            }
            video_url = Some(highest_bitrate_mp4_url.to_string());
          }
          let media_item = TweetMedia {
            shortened_img_url,
            full_img_url,
            kind,
            video_url,
          };
          media.push(media_item);
        }
        Some(media)
      } else {
        None
      }
    };

    let urls = {
      let urls_json = tweet_json["entities"]["urls"].as_array().unwrap();
      if urls_json.len() != 0 {
        let mut urls: Vec<TweetURLs> = Vec::new();
        for url_json in urls_json {
          let item = TweetURLs {
            shortened_url: url_json["url"].as_str().unwrap().to_string(),
            full_url: url_json["expanded_url"].as_str().unwrap().to_string(),
          };
          urls.push(item);
        }
        Some(urls)
      } else {
        None
      }
    };

    let quoted_tweet_id = {
      if let Some(quoted_tweet_id) = tweet_json["quoted_status_id_str"].as_str() {
        Some(quoted_tweet_id.to_string())
      } else {
        None
      }
    };

    let thread_id = {
      if let Some(thread_id) = tweet_json["self_thread"].as_str() {
        Some(thread_id.to_string())
      } else {
        None
      }
    };

    let retweet_tweet_id = {
      if let Some(retweet_tweet_id) = tweet_json["retweeted_status_id_str"].as_str() {
        Some(retweet_tweet_id.to_string())
      } else {
        None
      }
    };

    let date = tweet_json["created_at"].as_str().unwrap().to_string();

    let parsed_tweet = QueryTweet {
      id,
      user,
      text,
      media,
      urls,
      quote: None,
      thread_id,
      date,
      quoted_tweet_id,
      retweet_tweet_id,
      retweeted_by: None,
    };
    parsed_tweets.push(parsed_tweet);
  }

  /*
  retweets have an item for the tweet and an item for the retweet, though it 
  seems the main difference is that the retweet tweet.text starts with
  "RT @user: ", where user is the user of the tweet, not the retweeter. thus, 
  the retweet is essentially a duplicate, sowe can delete the retweet items

  though, it might be nice to know it is a retweet, so add a retweetedBy 
  property to the tweet
  the retweet has the property retweeted_status_id_str, which is the id of the
  retweeted tweet, and user_id_str, which is the id of the user that retweeted
  */

  let mut tweets_minus_retweet_dupes: Vec<QueryTweet> = Vec::new();
  #[derive(Clone)]
  struct RetweetItemInfo {
    id: String,
    user: String,
  }
  let mut track_retweets: Vec<RetweetItemInfo> = Vec::new();

  for parsed_tweet in parsed_tweets {
    // if a retweet, so something
    if let Some(retweet_tweet_id) = parsed_tweet.retweet_tweet_id {
      // track the id and user of the retweet, so we can assign this info 
      // to the original tweet
      let id = retweet_tweet_id;
      let user = parsed_tweet.user;
      track_retweets.push(RetweetItemInfo{ id, user });
      // don't add to new list of tweets
    } else {
      // if not a retweet, add to new list of tweets
      tweets_minus_retweet_dupes.push(parsed_tweet);
    }
  }
  parsed_tweets = tweets_minus_retweet_dupes;

  // add user retweet info to original tweet
  let mut tweets_incl_retweeted_by: Vec<QueryTweet> = Vec::new();

  for mut parsed_tweet in parsed_tweets.clone() {
    let matches: Vec<RetweetItemInfo> = track_retweets.clone().into_iter()
      .filter(|x| x.id == parsed_tweet.id)
      .collect();

    if matches.len() != 0 {
      // TODO: MAKE SURE THIS ACTUALLY MODIFIES THE `parsed_tweets` VEC
      parsed_tweet.retweeted_by = Some(
        matches.into_iter().map(|x| x.user).collect()
      );
    }
    tweets_incl_retweeted_by.push(parsed_tweet);
  }
  parsed_tweets = tweets_incl_retweeted_by;

  /*
  quote tweet items do not contain their quoted tweet, instead the quoted 
  tweet is its own item. thus, we must manually assign quoted tweets to their 
  quote tweet. 

  quoted tweets can be deleted, UNLESS that quoted tweet is from the same user
  as the feed, as in the quoted tweet item is both a quoted tweet and an 
  original tweet


  loop through until find tweet with quote. add the quote tweet to it, then
  add to parsedTweets.
  bad idea to remove from array mid-loop, so tracking quoted tweets for second
  for-loop that removes quoted tweets (again, only if quoted tweet not from 
  same user as the feed)
  */

  // attach quoted tweet to quote tweet, and track which tweets are quoted by 
  // tweet id
  let mut quoted_ids: Vec<String> = Vec::new();
  let mut temp_tweets: Vec<QueryTweet> = Vec::new();

  for mut parsed_tweet in parsed_tweets.clone() {
    if let Some(quoted_tweet_id) = parsed_tweet.quoted_tweet_id.clone() {
      // get first tweet from parsed_tweets where it's id matches quoted_tweet_id
      let quoted_tweet = parsed_tweets.clone().into_iter()
        .find(|t| t.id == quoted_tweet_id);
      if let Some(quoted_tweet) = quoted_tweet.clone() {
        // add quote tweet to tweet that quotes it
        parsed_tweet.quote = Some(Quote {
          id: quoted_tweet.id.clone(),
          user: quoted_tweet.user.clone(),
          text: quoted_tweet.text.clone(),
          media: quoted_tweet.media.clone(),
          urls: quoted_tweet.urls.clone(),
          thread_id: quoted_tweet.thread_id.clone(),
        });

        // track added tweets, UNLESS TWEET THAT IS QUOTED IS BY SAME 
        // USER OF FEED (e.g. from:elonmusk)
        // (see comment above initiation of trackTweetIDsOfAdded)

        // get array of users of the feed
        // allowed chars in twitter name are same as `\w`:
        // https://web.archive.org/web/20210506165356/https://www.techwalla.com/articles/what-characters-are-allowed-in-a-twitter-name
        let regex = Regex::new(r"from:\w+").unwrap();
        let query_users: Vec<String> = regex.find_iter(query)
          //                           slice to remove `from:` of each match
          .filter_map(|v| Some((&(v.as_str())[5..]).to_string()))
          .collect();
        // add quoted tweet id to array of quoted tweet ids if different 
        // user to feed user
        let is_diff_user = ! query_users.contains(&quoted_tweet.user);
        if is_diff_user {
          quoted_ids.push(quoted_tweet.clone().id);
        }
      }
    }
    temp_tweets.push(parsed_tweet.clone());
  }
  parsed_tweets = temp_tweets;

  // remove tweets that are quoted by other tweets
  temp_tweets = Vec::new();
  for parsed_tweet in parsed_tweets.clone() {
    // if tweet id is in quoted_ids, it is a quoted tweet to be removed
    if quoted_ids.contains(&parsed_tweet.id) {
      // don't add it
    } else {
      temp_tweets.push(parsed_tweet);
    }
  }
  parsed_tweets = temp_tweets;

  parsed_tweets
}

#[test]
fn test() {
  println!("{:?}", query_to_tweets("from:elonmusk"));
}
