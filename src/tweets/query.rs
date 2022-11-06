use crate::{
  fetch::query_fetch,
  types::{Tweet, TweetExtra},
};
use super::parsing::{parse_urls, parse_media};
use std::collections::HashMap;

/// get tweets from twitter search query
pub async fn query_to_tweets(query: &str) -> Vec<Tweet> {
  let mut parsed_tweets: Vec<Tweet> = Vec::new();

  let fetch_json = query_fetch(query).await;
  
  // data is separated into users and tweets, so to attach username to tweet, 
  // need to get user info first
  
  /* -------------------------------- users -------------------------------- */
  let users_json = match fetch_json["users"].as_object() {
    Some(users) => users,
    None => return parsed_tweets,
  };
  let mut user_id_to_name_map: HashMap<&str, &str> = HashMap::new();
  for (_, user_json) in users_json {
    let id = user_json["id_str"].as_str().unwrap();
    let name = user_json["screen_name"].as_str().unwrap();
    user_id_to_name_map.insert(id, name);
  }

  let tweets_json = fetch_json["tweets"].as_object().unwrap();

  for (_, tweet_json) in tweets_json {
    let id = tweet_json["id_str"].as_str().unwrap().to_string();
    let user = user_id_to_name_map[tweet_json["user_id_str"].as_str().unwrap()].to_string();
    let text = tweet_json["full_text"].as_str().unwrap().to_string();

    let media = parse_media(tweet_json);

    let urls = parse_urls(tweet_json);

    let quoted_tweet_id = match tweet_json["quoted_status_id_str"].as_str() {
      Some(quoted_tweet_id) => Some(quoted_tweet_id.to_string()),
      None => None,
    };

    let thread_id = match tweet_json["self_thread"].as_object() {
      Some(thread_id) => Some(thread_id["id_str"].as_str().unwrap().to_string()),
      None => None,
    };

    let retweet_tweet_id = match tweet_json["retweeted_status_id_str"].as_str(){
      Some(retweet_tweet_id) => Some(retweet_tweet_id.to_string()),
      None => None,
    };

    let date = tweet_json["created_at"].as_str().unwrap().to_string();

    let faves = tweet_json["favorite_count"].as_u64().unwrap();

    let parsed_tweet = Tweet {
      id,
      user,
      text,
      media,
      urls,
      quote: None,
      thread_id,
      extra: Some(TweetExtra {
        date,
        quoted_tweet_id,
        retweet_tweet_id,
        retweeted_by: None,
        faves,
      }),
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

  let mut tweets_minus_retweet_dupes: Vec<Tweet> = Vec::new();
  #[derive(Clone)]
  struct RetweetItemInfo {
    id: String,
    user: String,
  }
  let mut track_retweets: Vec<RetweetItemInfo> = Vec::new();

  for parsed_tweet in parsed_tweets {
    // if a retweet, so something
    if let Some(retweet_tweet_id) = parsed_tweet.extra.clone().unwrap().retweet_tweet_id {
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
  let mut tweets_incl_retweeted_by: Vec<Tweet> = Vec::new();

  for mut parsed_tweet in parsed_tweets.clone() {
    let matches: Vec<RetweetItemInfo> = track_retweets.clone().into_iter()
      .filter(|x| x.id == parsed_tweet.id)
      .collect();

    if matches.len() != 0 {
      // TODO: MAKE SURE THIS ACTUALLY MODIFIES THE `parsed_tweets` VEC
      parsed_tweet.extra.as_mut().unwrap().retweeted_by = Some(
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
  let mut temp_tweets: Vec<Tweet> = Vec::new();

  for mut parsed_tweet in parsed_tweets.clone() {
    if let Some(quoted_tweet_id) = parsed_tweet.extra.clone().unwrap().quoted_tweet_id.as_ref().clone() {
      // get first tweet from parsed_tweets where it's id matches quoted_tweet_id
      let quoted_tweet = parsed_tweets.clone().into_iter()
        .find(|t| &t.id == quoted_tweet_id);
      if let Some(quoted_tweet) = quoted_tweet.clone() {
        // add quote tweet to tweet that quotes it
        parsed_tweet.quote = Some(Box::new(Tweet {
          id: quoted_tweet.id.clone(),
          user: quoted_tweet.user.clone(),
          text: quoted_tweet.text.clone(),
          media: quoted_tweet.media.clone(),
          urls: quoted_tweet.urls.clone(),
          thread_id: quoted_tweet.thread_id.clone(),
          quote: None,
          extra: None,
        }));

        // track added tweets, UNLESS TWEET THAT IS QUOTED IS BY SAME 
        // USER OF FEED (e.g. from:elonmusk)
        // (see comment above initiation of trackTweetIDsOfAdded)

        // get array of users of the feed
        let query_users: Vec<String> = query_to_query_users(query);

        // add quoted tweet id to array of quoted tweet ids if different 
        // user to feed user
        let is_diff_user = ! query_users.contains(&quoted_tweet.user);
        if is_diff_user {
          quoted_ids.push((&quoted_tweet).id.clone());
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

/// extract the usernames from the search query
pub fn query_to_query_users(query: &str) -> Vec<String> {
  // allowed chars in twitter name are same as `\w`:
  // https://web.archive.org/web/20210506165356/https://www.techwalla.com/articles/what-characters-are-allowed-in-a-twitter-name
  // let regex = Regex::new(r"from:\w+").unwrap();
  // let query_users: Vec<String> = regex.find_iter(query)
  //   //                           slice to remove `from:` of each match
  //   .filter_map(|v| Some((&(v.as_str())[5..]).to_string()))
  //   .collect();
  let mut query_users = Vec::new();
  let mut collecting_name = false;
  let mut user_buf = String::new();
  let mut detect_buf = String::from("     ");
  for char in query.chars() {
    if collecting_name {
        let is_alphanumeric = match char {
          'a'..='z' | 'A'..='Z' | '0'..='9' => true,
          _ => false,
        };
        if is_alphanumeric {
          user_buf.push(char);
        } else {
          collecting_name = false;
          query_users.push(user_buf);
          user_buf = String::new();
        }
    } else {
      // store last 5 chars as `detect_buf` to match "from:" when hit a ":"
      detect_buf.remove(0);
      detect_buf.push(char);
      if char.to_string() == ":" && detect_buf == "from:" {
        collecting_name = true;
      }
    }
  }
  // in the case that the username ends at end of string, need to add the name to vec
  if collecting_name {
      query_users.push(user_buf);
  }
  query_users
}
