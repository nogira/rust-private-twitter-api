use std::{collections::HashSet, error::Error};
use serde_json::Value;
use tokio::time::{sleep, Duration};
use crate::{
  fetch::id_fetch,
  types::Tweet,
  tweets::parsing::{parse_urls, parse_media},
};

pub async fn url_to_tweets(url: &str) -> Result<Vec<Tweet>, Box<dyn Error>> {
  let tweet_id = url.split("/").collect::<Vec<&str>>()[5];
  let mut tweets = url_to_tweets_no_cursor_position(tweet_id).await?;

  // if tweet thread has not finished, change cursor position to get next 
  // tweets. loop until have all tweets
  // if the tweet_item is a "show more" button, i added it as a tweet where 
  // the `id` is "more_tweets_in_thread", and the `text` is the "show more" 
  // cursor position
  let mut last_tweet = &tweets[tweets.len() -1];
  while &last_tweet.id == "more_tweets_in_thread" {
    let cursor = &last_tweet.text.clone();
    // rm the "show more" temp tweet (last tweet) bc don't need anymore
    tweets.pop();

    // get extra tweets past "show more"
    sleep(Duration::from_millis(200)).await; // wait between requests
    let show_more_tweets = url_to_tweets_with_cursor_position(tweet_id, cursor.as_str()).await?;
    
    // add tweets, checking to make sure they are unique
    let existing_tweet_ids = tweets.iter()
      .map(|t| t.id.clone()).collect::<HashSet<String>>();
    for show_more_tweet in show_more_tweets {
      // if tweet is not already in `tweets`, add it
      if ! existing_tweet_ids.contains(&show_more_tweet.id) {
        tweets.push(show_more_tweet);
      }
    }
    // get last tweet so while loop can check if it is a "show_more"
    last_tweet = &tweets[tweets.len() -1];
  }
  Ok(tweets)
}

async fn url_to_tweets_with_cursor_position(tweet_id: &str, cursor: &str
) -> Result<Vec<Tweet>, Box<dyn Error>> {
  let tweet_groups_json = id_fetch(tweet_id, cursor, false).await?;
  let tweet_group = tweet_groups_json.as_array().unwrap();
  Ok(tweet_group_to_tweets(&tweet_group))
}

/// get a tweet/tweet-thread in a parsed format (most of the junk removed), as a
/// list of tweets, starting with the first tweet
/// 
/// if more information is required than in the struct `Tweet`, use id_fetch()` 
/// instead
async fn url_to_tweets_no_cursor_position(tweet_id: &str
) -> Result<Vec<Tweet>, Box<dyn Error>> {
  let tweet_groups_json = id_fetch(
    tweet_id, "", false).await?;
  let tweet_groups = tweet_groups_json.as_array().unwrap();

  // find out which tweet group contains the main tweet
  let main_tweet_index: usize = get_main_tweet_index(&tweet_groups, tweet_id);
  // get the main group tweets
  let mut main_group_tweets: Vec<Tweet> = tweet_group_to_tweet_or_tweets(&tweet_groups[main_tweet_index]);

  /* ---- Examples of tweet patterns we need to match ----

  users: A, B, C, D

  // original tweet
  1: (A) ->  B  ->  C  ->  D    (single tweet)
  3: (A) ->  A  ->  A  ->  B    (start tweet thread)
  4:  A  -> (A) ->  A  ->  B    (mid tweet thread)
  5:  A  ->  A  -> (A) ->  B    (end tweet thread)

  // reply
  2:  A  -> (B) ->  C  ->  D    (single reply)
  6:  A  -> (B) ->  B  ->  B    (start reply thread)
  7:  A  ->  B  -> (B) ->  B    (mid reply thread)
  8:  A  ->  B  ->  B  -> (B)   (end reply thread)

  TWO TYPES OF TWEETS WE NEED TO PARSE:
  1) tweet group at position 0 OR with diff user in prev tweet group
    - if next tweet group is diff user, just return main tweet group
    - if next tweet group is same user, return main tweet group, AND next tweet
      group (thread)
  2) tweet group with same user prev to main tweet group. this is either mid 
     or end of thread/reply-thread
    - for this, just return main tweet group
  */

  // if there is a next tweet group, get it
  // need to use `.get()` bc there might not be any replies to the main tweet
  let mut next_group_tweets: Vec<Tweet> = match tweet_groups.get(main_tweet_index + 1) {
    Some(next_group) => tweet_group_to_tweet_or_tweets(next_group),
    None => Vec::new(),
  };

  /* -------------------IF MAIN TWEET IN FIRST TWEET GROUP------------------- */
  // A) IT IS A SINGLE TWEET
  // B) IT IS A SINGLE TWEET PLUS THE THREAD ENTENDING FROM THE SINGLE TWEET
  //
  // if main tweet is first tweet, return first tweet group (main tweet), and 
  // second tweetGroup (the thread) if it is same user
  if main_tweet_index == 0 {
    // IF NEXT TWEET GROUP IS GREATER THAN ZERO (required to be able to get user)
    // AND USER IS SAME AS MAIN TWEET, IT MUST BE THE THREAD, SO APPEND TO ALL_PARSED_TWEETS
    if next_group_tweets.len() > 0 && next_group_tweets[0].user == main_group_tweets[0].user {
      main_group_tweets.append(&mut next_group_tweets);
    }
    return Ok(main_group_tweets);
  }

  /* ---------------IF MAIN TWEET **NOT** IN FIRST TWEET GROUP--------------- */
  // A) TWEET IS MID/END-THREAD IF BOTH PREV TWEET GROUP AND NEXT TWEET GROUP ARE 
  // SAME USER AS MAIN TWEET GROUP (OR INSTEAD OF SAME USER, NEXT IS MISSING)
  // B) TWEET IS SINGLE REPLY IF PREV IS DIFF USER AND POST IS MISSING OR DIFF USER
  // C) TWEET IS THREADED REPLY IF PREV IS DIFF USER AND POST IS SAME USER

  let prev_tweet_is_same_user = {
    let prev_group_tweets: Vec<Tweet> = tweet_group_to_tweet_or_tweets(&tweet_groups[main_tweet_index - 1]);
    prev_group_tweets[0].user == main_group_tweets[0].user
  };

  // if prev tweet group is same user, it is mid/end of tweet thread, so just 
  // return main tweet group (which is a single tweet)
  if prev_tweet_is_same_user {
    return Ok(main_group_tweets);

  // if prev tweet group is diff user, its first tweet of a reply
  } else {
    // add thread if exists
    if next_group_tweets.len() > 0
    && next_group_tweets[0].user == main_group_tweets[0].user {
      main_group_tweets.append(&mut next_group_tweets);
    }
    return Ok(main_group_tweets);
  }
}

fn get_main_tweet_index(tweet_groups: &Vec<Value>, tweet_id: &str) -> usize {
  for (i, tweet_group) in tweet_groups.clone().iter().enumerate() {
    let entry_id = tweet_group["entryId"].as_str().unwrap();
    // "tweet-1516856286738598375" -> "1516856286738598375"
    let id = &entry_id[6..];
    if id == tweet_id {
      return i;
    }
  }
  return 0; // will never reach this return, but rust complains if it isn't there
}

/// get the tweet/tweets from a tweet group
/// 
/// the tweet group is either a single tweet, or multiple tweets
fn tweet_group_to_tweet_or_tweets(tweet_group: &Value) -> Vec<Tweet> {
  match tweet_group.get("content")
  .and_then(|v| v.get("items")).and_then(|v| v.as_array()) {
    /* ------if group has items (I.E. TWEET GROUP HAS MULTIPLE TWEETS)------ */
    Some(contents) => tweet_group_to_tweets(contents),
    /* ------if group has no items (I.E. TWEET GROUP IS JUST ONE TWEET)------ */
    None => match parse_tweet_contents(&tweet_group["content"]["itemContent"]) {
      Some(tweet) => Vec::from([tweet]),
      None => Vec::new(),
    },
  }
}

/// loop through json tweet items to get parsed tweets
fn tweet_group_to_tweets(tweet_group: &Vec<Value>) -> Vec<Tweet> {
  tweet_group.iter().map(|tweet_item| {
    parse_tweet_contents(&tweet_item["item"]["itemContent"]).unwrap()
  }).collect()
}

/// convert a single tweet object to a `Tweet`
fn parse_tweet_contents(unparsed_tweet: &Value) -> Option<Tweet> {
  let unparsed_tweet = match unparsed_tweet
  // normal tweet
  .get("tweet_results").and_then(|v| v.get("result"))
  // quote tweet
  .or(unparsed_tweet.get("result")) {
    // if tweet
    Some(unparsed_tweet) => {
      let kind = item_type(unparsed_tweet);
      match kind.as_str() {
        // normal visible tweet
        "Tweet" => unparsed_tweet,
        // idk why this happens, but the example is in `url_test_text_only_tweets_15()`
        // and `url_test_text_only_tweets_16()`
        // (normal-ish tweet)
        "TweetWithVisibilityResults" => &unparsed_tweet["tweet"],
        "TweetTombstone" => {
          // if tweet is unable to be viewed (e.g. "You’re unable to view this Tweet 
          // because this account owner limits who can view their Tweets. Learn more"), 
          // unparsed_tweet["legacy"] will equal null, but we still want to tell the 
          // user this tweet is missing, so we must create our own tweet
          //
          // FIXME: it is possible this will not give the desirable behavior if 
          // the non-viewable tweet is a deleted tweet, and is from the same 
          // user as the main tweet (while this is the first tweet in the 
          // thread), thus the user match check will assume it is not the same 
          // user, and not add ANY of the thread tweets, but we still want the 
          // thread tweets
          // perhaps i just need to check the next tweet if first tweet checked 
          // has user="hidden"
          return create_missing_tweet(unparsed_tweet);
        },
        _ => panic!("idk what type this is: {kind}"),
      }
    },
    // if "Show more" button
    None =>  {
      // if its a "show more" item, add as special last tweet (to signal we need 
      // a new request at the cursor position), then break
      let kind = item_type(unparsed_tweet);
      if kind == "TimelineTimelineCursor".to_string() {
        let show_more_cursor = unparsed_tweet["value"].as_str().unwrap().to_string();
        return Some(Tweet {
          id: "more_tweets_in_thread".to_string(),
          user: "".to_string(),
          text: show_more_cursor, 
          media: None, urls: None, quote: None, thread_id: None, extra: None
        });
      } else {
        // FIXME: does this ever trigger??? (since i unwrap all 
        // `Option<Tweet>`s, i should find out soon enough)
        return None;
      }
    },
  };
  let id = unparsed_tweet["legacy"]["id_str"].as_str().unwrap().to_string();
  let user = unparsed_tweet["core"]["user_results"]["result"]["legacy"]["screen_name"].as_str().unwrap().to_string();
  let text = unparsed_tweet["legacy"]["full_text"].as_str().unwrap().to_string();
  let media = parse_media(&unparsed_tweet["legacy"]);
  let urls = parse_urls(&unparsed_tweet["legacy"]);
  let quote = unparsed_tweet.get("quoted_status_result")
    .and_then(|quote_contents| parse_tweet_contents(quote_contents))
    .and_then(|tweet| Some(Box::new(tweet)));
  let thread_id = unparsed_tweet["legacy"].get("self_thread")
    .and_then(|o| Some(o.get("id_str").unwrap().as_str().unwrap().to_string()));
  return Some(Tweet { id, user, text, media, urls, quote, thread_id, extra: None })
}

/// get the type of item in twitter raw json
/// 
/// note: this will be unable to find a type of a quoted tweet
fn item_type(item: &Value) -> String {
  match item["entryType"].as_str()
  .or(item["itemType"].as_str())
  // it seems typename is not returned from my requests, but does in 
  // webinspector so i prob have some header option turned off
  .or(item["__typename"].as_str()) {
    Some(v) => v,
    None => panic!("can't find type:\n{item}"),
  }.to_string()
}

fn create_missing_tweet(unparsed_tweet: &Value) -> Option<Tweet> {
  let txt = unparsed_tweet["tombstone"]["text"]["text"].as_str().unwrap();
  Some(Tweet {
    id: "".to_string(),
    user: "unknown".to_string(),
    // slice is to remove " Learn more"
    text: format!("<<< {} >>>", &txt[..(txt.len() - 11)]),
    media: None, urls: None, quote: None, thread_id: None, extra: None,
  })
}

/* ----------------------- url_to_recommended_tweets ----------------------- */

pub async fn url_to_recommended_tweets(url: &str) -> Result<Vec<Tweet>, Box<dyn Error>> {
  let id_from_input_url = url.split("/").collect::<Vec<&str>>()[5];
  let tweet_groups_json = id_fetch(&id_from_input_url, 
    "", true).await?;
  let tweet_groups = tweet_groups_json.as_array().unwrap();

  // all recommended tweets are in second-last tweet_group item
  let recommended_tweets = tweet_groups[&tweet_groups.len() - 2]
    .get("content").and_then(|v| v.get("items"))
    .and_then(|v| v.as_array()).unwrap();
  
  Ok(tweet_group_to_tweets(recommended_tweets))
}
