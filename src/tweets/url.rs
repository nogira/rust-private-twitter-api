use crate::{
  fetch::id_fetch,
  types::{Tweet, TweetMedia, TweetURLs, Quote},
};
use serde_json::Value;
use tokio::time::{sleep, Duration};

pub async fn url_to_tweets(url: &str) -> Vec<Tweet> {
  let mut tweets = url_to_tweets_no_cursor_position(url).await;

  // if tweet thread has not finished, change cursor position to get next 
  // tweets. loop until have all tweets
  let mut last_tweet = &tweets[tweets.len() -1];
  while &last_tweet.id == &"more_tweets_in_thread".to_string() {
    // if the tweet_item is a "show more" button, i added it as a tweet where 
    // the `id` is "more_tweets_in_thread", and the `text` is the "show more" 
    // cursor position

    // wait between requests
    sleep(Duration::from_millis(100)).await;

    let cursor = &last_tweet.text.clone();
    // rm last tweet
    tweets.pop();

    // get extra tweets past "show more"
    let show_more_tweets = url_to_tweets_with_cursor_position(url, cursor.as_str()).await;
    
    // add tweets, checking to make sure they are unique
    for show_more_tweet in show_more_tweets {
      let tweet_is_duplicate = tweets.iter()
        .any(|tweet| &tweet.id == &show_more_tweet.id);
      if ! tweet_is_duplicate {
        tweets.push(show_more_tweet);
      }
    }
    //get last tweet so while loop can check if show_more
    last_tweet = &tweets[tweets.len() -1];
  }
  tweets
}

async fn url_to_tweets_with_cursor_position(url: &str, cursor: &str) -> Vec<Tweet> {
  let id_from_input_url = url.split("/").collect::<Vec<&str>>()[5];
  let tweet_groups_json = id_fetch(id_from_input_url, cursor, false).await.unwrap();
  let tweet_groups = &tweet_groups_json.as_array().unwrap();
  let all_parsed_tweets: Vec<Tweet> = tweet_module_group_to_tweets(&tweet_groups);

  all_parsed_tweets
}

/// get a tweet/tweet-thread in a parsed format (most of the junk removed), as a
/// list of tweets, starting with the first tweet
/// 
/// if more information is required than in the struct `Tweet`, use id_fetch()` 
/// instead
async fn url_to_tweets_no_cursor_position(url: &str) -> Vec<Tweet> {
  let id_from_input_url = url.split("/").collect::<Vec<&str>>()[5];
  let tweet_groups_json = id_fetch(
    id_from_input_url, "", false
  ).await.unwrap();
  let tweet_groups = &tweet_groups_json.as_array().unwrap();
  let mut all_parsed_tweets: Vec<Tweet> = Vec::new();

  // find out which tweet group contains the main tweet
  let main_tweet_index: usize = get_main_tweet_index(&tweet_groups, id_from_input_url);
  // get the main group tweets
  let mut main_group_tweets: Vec<Tweet> = tweet_group_to_tweets(&tweet_groups[main_tweet_index]);

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
  let mut next_tweet_group: Vec<Tweet> = Vec::new();

  let next_group = &tweet_groups[main_tweet_index + 1];
  // check next group exists
  if next_group.is_object() {
    // if the tweet group is not the show more button, add tweet group
    if next_group.get("content")
      .and_then(|v| v.get("itemContent"))
      .and_then(|v| v.get("cursorType"))
      .and_then(|v| v.as_str()).unwrap_or("fail") != "ShowMoreThreadsPrompt" {
      next_tweet_group = tweet_group_to_tweets(next_group);
    }
  };
  
  // if main tweet is first tweet, add first tweetGroup (main tweet), and 
  // second tweetGroup (the thread) if it is same user, to allParsedTweets
  if main_tweet_index == 0 {
    let main_tweet_user = &main_group_tweets[0].user.clone();
    // below line removes all tweets from `main_tweet`, so need to grab 
    // `main_tweet_user` in advance of 5 lines down
    all_parsed_tweets.append(&mut main_group_tweets);
    if next_tweet_group.len() > 0 {
        let mut thread: Vec<Tweet> = next_tweet_group;
        // this is also false if thread doesn't exist
        if thread.len() > 0 && &thread[0].user == main_tweet_user {
            all_parsed_tweets.append(&mut thread);
        }
    }
    return all_parsed_tweets;
  }

  // get prev tweet group and next tweet group
  let prev_tweet_group: Vec<Tweet> = tweet_group_to_tweets(&tweet_groups[main_tweet_index - 1]);
  let prev_tweet_is_same_user = prev_tweet_group[0].user == main_group_tweets[0].user;

  // if prev tweet group is diff user, its first tweet of a reply
  if ! prev_tweet_is_same_user {
    let main_tweet_user = &main_group_tweets[0].user.clone();
    // let i = main_tweet_index; // <-- 🚨🚨🚨 y tf is this here?? just commented out, but leaving bc might be there for a reason
    all_parsed_tweets.append(&mut main_group_tweets);
    if next_tweet_group.len() > 0 {
        let mut thread = next_tweet_group;
        // this is also false if thread doesn't exist
        if thread.len() > 0 && &thread[0].user == main_tweet_user {
          all_parsed_tweets.append(&mut thread);
        }
    }
    return all_parsed_tweets;
  }

  // if prev tweet group is same user, it is mid/end of tweet thread, so just 
  // return main tweet group
  if prev_tweet_is_same_user {
    all_parsed_tweets.append(&mut main_group_tweets);
  }

  all_parsed_tweets
}

fn get_main_tweet_index(tweet_groups: &Vec<Value>, id_from_input_url: &str) -> usize {
  let mut i = 0;
  for tweet_group in tweet_groups.clone() {
    let entry_id = tweet_group["entryId"].as_str().unwrap();
    // "tweet-1516856286738598375" -> "1516856286738598375"
    let id = &entry_id[6..];
    if id == id_from_input_url {
      return i;
    }
    i += 1;
  }
  // will never reach this return, but rust complains if it isn't there
  return 0;
}

fn tweet_group_to_tweets(tweet_group: &Value) -> Vec<Tweet> {
  let mut return_tweets: Vec<Tweet> = Vec::new();
  let contents = tweet_group.get("content")
    .and_then(|v| v.get("items"))
    .and_then(|v| v.as_array());

  // if `contents` is not empty
  if let Some(contents) = contents {
    let mut tweets = tweet_module_group_to_tweets(contents);

    if tweets.len() > 0 {
      return_tweets.append(&mut tweets);
    }

  // if `contents` is empty
  } else {
    // tweet item group to tweet
    let tweet = parse_tweet_contents(&tweet_group["content"]);

    if let Some(tweet) = tweet {
      return_tweets.push(tweet);
    }
  }
  return return_tweets;
}

fn tweet_module_group_to_tweets(tweet_group: &Vec<Value>) -> Vec<Tweet> {
  let mut tweets: Vec<Tweet> = Vec::new();
  let tweet_group = tweet_group.clone();

  for tweet_item in tweet_group {
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
        media: None, urls: None, quote: None, thread_id: None
      });
      break;
    }
    let parsed_tweet: Option<Tweet> = parse_tweet_contents(unparsed_tweet);
    tweets.push(parsed_tweet.unwrap());
  }
  return tweets;
}


/// return `Tweet`, `Quote`, or `None`
fn parse_tweet_contents(unparsed_tweet: &Value) -> Option<Tweet> {
  let unparsed_tweet = match unparsed_tweet.get("itemContent")
    .and_then(|v| v.get("tweet_results"))
    .and_then(|v| v.get("result")) {
    Some(v) => v,
    // handle quote tweet (if normal tweet (above) returns null)
    None => match unparsed_tweet.get("result") {
      Some(v) => v,
      // if the tweet_item is a "Show more" button, it has no `result` attr, so 
      // above will return `None`. if so, it's not a tweet, so return `None`
      // FIXME: does this ever trigger bc i handle "show more"s in `tweet_module_group_to_tweets`
      None => return None,
    },
  };

  let id =  unparsed_tweet["legacy"]["id_str"].as_str().unwrap().to_string();
  let user = unparsed_tweet["core"]["user_results"]["result"]["legacy"]["screen_name"].as_str().unwrap().to_string();
  let text = unparsed_tweet["legacy"]["full_text"].as_str().unwrap().to_string();

  let media = match unparsed_tweet["legacy"]["entities"]["media"].as_array() {
    Some(media_json) => {
      let mut media: Vec<TweetMedia> = Vec::new();
      for img in media_json {
          let item = TweetMedia {
              shortened_img_url: img["url"].as_str().unwrap().to_string(),
              full_img_url: img["media_url_https"].as_str().unwrap().to_string(),
              kind: img["type"].as_str().unwrap().to_string(), // photo or video
              video_url: None, // FIXME: implement video parsing
          };
          media.push(item);
      }
      Some(media)
    },
    None => None,
  };

  let urls = match unparsed_tweet["legacy"]["entities"]["urls"].as_array() {
    Some(urls_json) => {
      let mut urls: Vec<TweetURLs> = Vec::new();
      for url in urls_json {
          let item = TweetURLs {
              shortened_url: url["url"].as_str().unwrap().to_string(),
              full_url: url["expanded_url"].as_str().unwrap().to_string(),
          };
          urls.push(item);
      }
      Some(urls)
    },
    None => None,
  };

  let quote: Option<Quote> = match unparsed_tweet.get("quoted_status_result") {
    Some(quote_contents) => {
      if let Some(tweet) = parse_tweet_contents(quote_contents) {
        let quote = Quote {
          id: tweet.id,
          user: tweet.user,
          text: tweet.text,
          media: tweet.media,
          urls: tweet.urls,
          thread_id: tweet.thread_id
        };
        // not sure y tf i need these 2 lines of code ?????
        // mainTweet.quote.url = tweetContents.legacy.quoted_status_permalink
        // delete mainTweet.quote.url.display
        Some(quote)
      } else {
        None
      }
    },
    None => None
  };

  return Some(Tweet {
    id,
    user,
    text,
    media,
    urls,
    quote,
    thread_id: None,
  })
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

// #[tokio::test]
// async fn recommended_tweets_test() {
//   println!("url_to_tweets()  //  thread, 1st-tweet");
//   let url = "https://twitter.com/epolynya/status/1513868637307691009";
//   let tweets = url_to_recommended_tweets(url).await;
//   // println!("{:?}", tweets);
//   assert!(tweets.len() > 10); // should be 24, so i prob need to join cursor query
// }