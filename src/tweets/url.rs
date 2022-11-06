use std::collections::HashSet;

use crate::{
  fetch::id_fetch,
  types::Tweet,
};
use super::parsing::{parse_urls, parse_media};
use serde_json::Value;
use tokio::time::{sleep, Duration};

pub async fn url_to_tweets(url: &str) -> Vec<Tweet> {
  let tweet_id = url.split("/").collect::<Vec<&str>>()[5];
  let mut tweets = url_to_tweets_no_cursor_position(tweet_id).await;

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
    sleep(Duration::from_millis(100)).await; // wait between requests
    let show_more_tweets = url_to_tweets_with_cursor_position(tweet_id, cursor.as_str()).await;
    
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
  tweets
}

async fn url_to_tweets_with_cursor_position(tweet_id: &str, cursor: &str) -> Vec<Tweet> {
  let tweet_groups_json = id_fetch(tweet_id, cursor, false).await.unwrap();
  let tweet_groups = tweet_groups_json.as_array().unwrap();
  let all_parsed_tweets: Vec<Tweet> = tweet_module_group_to_tweets(&tweet_groups);

  all_parsed_tweets
}

/// get a tweet/tweet-thread in a parsed format (most of the junk removed), as a
/// list of tweets, starting with the first tweet
/// 
/// if more information is required than in the struct `Tweet`, use id_fetch()` 
/// instead
async fn url_to_tweets_no_cursor_position(tweet_id: &str) -> Vec<Tweet> {
  let tweet_groups_json = id_fetch(
    tweet_id, "", false).await.unwrap();
  let tweet_groups = tweet_groups_json.as_array().unwrap();
  let mut all_parsed_tweets: Vec<Tweet> = Vec::new();

  // find out which tweet group contains the main tweet
  let main_tweet_index: usize = get_main_tweet_index(&tweet_groups, tweet_id);
  // get the main group tweets
  let mut main_group_tweets: Vec<Tweet> = tweet_group_to_tweets(&tweet_groups[main_tweet_index]);
  println!("!!: {:?}", main_group_tweets);

  /* ---- Examples of tweet patterns we need to match ----

  1: (user1) -> user2 -> user3 -> user4   (single tweet)

  2: user1 -> (user2) -> user3 -> user4   (single reply)

  3: (user1) -> user1 -> user1 -> user3   (start tweet thread)
  4: user1 -> (user1) -> user1 -> user3   (mid tweet thread)
  5: user1 -> user1 -> (user1) -> user3   (end tweet thread)

  6: user1 -> (user2) -> user2 -> user2   (start reply thread)
  7: user1 -> user2 -> (user2) -> user2   (mid reply thread)
  8: user1 -> user2 -> user2 -> (user2)   (end reply thread)

  TWO MAIN TYPES OF TWEETS WE NEED TO PARSE:
  1: tweet group at position 0 OR with diff user in prev tweet group
        - if next tweet group is diff user, just add main tweet group to 
          allParsedTweets
        - if next tweet group is same user, add main tweet group, AND next tweet
          group (thread) to allParsedTweets
  2: tweet group with same user prev to main tweet group. this is either mid 
      or end of thread/reply-thread
        - for this, just add main tweet group to allParsedTweets
  */

  // if there is a next tweet group, get it
  // need to use `.get()` bc there might not be any replies to the main tweet
  let mut next_tweet_group: Vec<Tweet> = match tweet_groups.get(main_tweet_index + 1) {
    Some(next_group) =>
      // // check next group exists
      // if next_group.is_object() && // FIXME: <<--- i think this is redundant
      // if the tweet group is not the show more button, add tweet group
      match next_group.get("content")
      .and_then(|v| v.get("itemContent"))
      .and_then(|v| v.get("cursorType"))
      .and_then(|v| v.as_str()).unwrap_or("fail") != "ShowMoreThreadsPrompt" {
        true => tweet_group_to_tweets(next_group),
        false => Vec::new(),
      },
    None => Vec::new(),
  };

  /* -------------------IF MAIN TWEET IN FIRST TWEET GROUP------------------- */
  // A) IT IS A SINGLE TWEET
  // B) IT IS A SINGLE TWEET PLUS THE THREAD ENTENDING FROM THE SINGLE TWEET
  //
  // if main tweet is first tweet, add first tweetGroup (main tweet), and 
  // second tweetGroup (the thread) if it is same user, to allParsedTweets
  if main_tweet_index == 0 {
    let main_tweet_user = &main_group_tweets[0].user.clone();
    // below line removes all tweets from `main_tweet`, so need to grab 
    // `main_tweet_user` in advance of 5 lines down
    all_parsed_tweets = main_group_tweets;
    // IF NEXT TWEET GROUP IS GREATER THAN ZERO (required to be able to get user)
    // AND USER IS SAME AS MAIN TWEET, IT MUST BE THE THREAD, SO APPEND TO ALL_PARSED_TWEETS
    if next_tweet_group.len() > 0 && &next_tweet_group[0].user == main_tweet_user {
      all_parsed_tweets.append(&mut next_tweet_group);
    }
    return all_parsed_tweets;
  }

  /* ---------------IF MAIN TWEET **NOT** IN FIRST TWEET GROUP--------------- */
  // A) TWEET IS MID/END-THREAD IF BOTH PREV TWEET GROUP AND NEXT TWEET GROUP ARE 
  // SAME USER AS MAIN TWEET GROUP (OR INSTEAD OF SAME USER, NEXT IS MISSING)
  // B) TWEET IS SINGLE REPLY IF PREV IS DIFF USER AND POST IS MISSING OR DIFF USER
  // C) TWEET IS THREADED REPLY IF PREV IS DIFF USER AND POST IS SAME USER

  // get prev tweet group
  let prev_tweet_group: Vec<Tweet> = tweet_group_to_tweets(&tweet_groups[main_tweet_index - 1]);
  let prev_tweet_is_same_user = prev_tweet_group[0].user == main_group_tweets[0].user;

  // if prev tweet group is same user, it is mid/end of tweet thread, so just 
  // return main tweet group (which is a single tweet)
  if prev_tweet_is_same_user {
    return main_group_tweets;

  // if prev tweet group is diff user, its first tweet of a reply
  } else {
    let main_tweet_user = &main_group_tweets[0].user.clone();
    all_parsed_tweets.append(&mut main_group_tweets);
    // add thread if exists
    if next_tweet_group.len() > 0 && &next_tweet_group[0].user == main_tweet_user {
      all_parsed_tweets.append(&mut next_tweet_group);
    }
    return all_parsed_tweets;
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
  // will never reach this return, but rust complains if it isn't there
  return 0;
}

/// get the tweet/tweets from a tweet group
fn tweet_group_to_tweets(tweet_group: &Value) -> Vec<Tweet> {
  println!("1");
  // the tweet group is either a single tweet, or multiple tweets

  /* -------if group has items (I.E. TWEET GROUP HAS MULTIPLE TWEETS)------- */
  if let Some(contents) = tweet_group.get("content")
    .and_then(|v| v.get("items")).and_then(|v| v.as_array()) {
    println!("1.5");
    tweet_module_group_to_tweets(contents)

  /* -------if group has no items (I.E. TWEET GROUP IS JUST ONE TWEET)------- */
  } else {
    match parse_tweet_contents(&tweet_group["content"]) {
      Some(tweet) => Vec::from([tweet]),
      None => Vec::new(),
    }
  }
}

fn tweet_module_group_to_tweets(tweet_group: &Vec<Value>) -> Vec<Tweet> {
  let mut tweets: Vec<Tweet> = Vec::new();

  for tweet_item in tweet_group.clone() {
    let unparsed_tweet = &tweet_item["item"];

    // if its a "show more" item, add as special last tweet (to signal we need 
    // a new request at the cursor position), then break
    if unparsed_tweet.get("itemContent")
      .and_then(|v| v.get("displayTreatment"))
      .and_then(|v| v.get("actionText"))
      .and_then(|v| v.as_str()).unwrap_or("fail") == "Show replies" {
      let show_more_cursor = unparsed_tweet["itemContent"]["value"].as_str().unwrap().to_string();
      tweets.push(Tweet {
        id: "more_tweets_in_thread".to_string(),
        user: "".to_string(),
        text: show_more_cursor, 
        media: None, urls: None, quote: None, thread_id: None, extra: None
      });
      break;
    }
    // if normal tweet, just add the normal tweet
    let parsed_tweet: Option<Tweet> = parse_tweet_contents(unparsed_tweet);
    tweets.push(parsed_tweet.unwrap());
  }
  return tweets;
}


/// convert a single tweet object to a `Tweet`
fn parse_tweet_contents(unparsed_tweet: &Value) -> Option<Tweet> {
  let unparsed_tweet = match unparsed_tweet.get("itemContent")
    .and_then(|v| v.get("tweet_results"))
    .and_then(|v| v.get("result")) {
    // if normal tweet
    Some(v) => v,
    // if quote tweet
    None => &unparsed_tweet["result"]
    // None => match unparsed_tweet.get("result") {
    //   Some(v) => v,
    //   // if the tweet_item is a "Show more" button, it has no `result` attr, so 
    //   // above will return `None`. if so, it's not a tweet, so return `None`
    //   // FIXME: does this ever trigger??? bc i handle "show more"s in `tweet_module_group_to_tweets`
    //   None => return None,
    // },
  };
  // if tweet is unable to be viewed (e.g. "You’re unable to view this Tweet 
  // because this account owner limits who can view their Tweets. Learn more"), 
  // unparsed_tweet["legacy"] will equal null, but we still want to tell the 
  // user this tweet is missing, so we must create out own tweet
  if unparsed_tweet["legacy"].is_null() {
    // for now, pretend there is no tweet
    // FIXME: in the tweet this fixes, it does no seem the tweet is a part of 
    // the thread, so we don't want to include it, but if the tweet *was* a 
    // part of the thread, we would want it, so how do i differentiate between 
    // the two??
    // return None;
    println!("2");
    return Some(Tweet {
      id: "".to_string(),
      user: "unknown".to_string(),
      text: format!("<<< {} >>>", unparsed_tweet["tombstone"]["text"]["text"].as_str().unwrap()),
      media: None,
      urls: None,
      quote: None,
      thread_id: None,
      extra: None,
    })
  }

  let id = unparsed_tweet["legacy"]["id_str"].as_str().unwrap().to_string();
  let user = unparsed_tweet["core"]["user_results"]["result"]["legacy"]["screen_name"].as_str().unwrap().to_string();
  let text = unparsed_tweet["legacy"]["full_text"].as_str().unwrap().to_string();
  let media = parse_media(&unparsed_tweet["legacy"]);
  let urls = parse_urls(&unparsed_tweet["legacy"]);
  let quote = match unparsed_tweet.get("quoted_status_result") {
    Some(quote_contents) => match parse_tweet_contents(quote_contents) {
      Some(tweet) => Some(Box::new(Tweet {
        id: tweet.id,      user: tweet.user,
        text: tweet.text,  media: tweet.media,
        urls: tweet.urls,  thread_id: tweet.thread_id,
        quote: None,       extra: None,
      })),
      None => None,
    },
    None => None,
  };

  return Some(Tweet { id, user, text, media, urls, quote, thread_id: None, extra: None })
}


/* ----------------------- url_to_recommended_tweets ----------------------- */

pub async fn url_to_recommended_tweets(url: &str) -> Vec<Tweet> {

  let id_from_input_url = url.split("/").collect::<Vec<&str>>()[5];
  let tweet_groups_json = id_fetch(&id_from_input_url, "", true).await.unwrap();
  let tweet_groups = tweet_groups_json.as_array().unwrap();
  let mut all_parsed_tweets: Vec<Tweet> = Vec::new();

  // all recommended tweets are in second-last tweetGroup item
  let recommended_tweets = tweet_groups[&tweet_groups.len() - 2].get("content").and_then(|v| v.get("items")).and_then(|v| v.as_array()).unwrap();
  for tweet in recommended_tweets {
      let tweet_contents = &tweet["item"];
      if let Some(parsed_tweet) = parse_tweet_contents(tweet_contents) {
          all_parsed_tweets.push(parsed_tweet);
      }
  }
  return all_parsed_tweets;
}
