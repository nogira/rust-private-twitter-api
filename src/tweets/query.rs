use crate::{
  fetch::query_fetch,
  types::{Tweet, TweetExtra},
};
use super::parsing::{parse_urls, parse_media};
use std::collections::HashMap;

/// get tweets from twitter search query
pub async fn query_to_tweets(query: &str) -> Result<Vec<Tweet>, Box<dyn std::error::Error>> {
  // key is the tweet item id, val is (tweet, quoted_tweet_id, retweeted_tweet_id)
  // quoted_tweet_id = the id of the tweet being quoted (to be able match quote 
  //   tweets to the quoted tweet)
  // retweeted_tweet_id = the id of the tweet being retweeted
  let mut parsed_tweets_map: HashMap<String, (Tweet, Option<String>, Option<String>)> = HashMap::new();

  let fetch_json = query_fetch(query).await?;
  
  // data is separated into users and tweets, so to attach username to tweet, 
  // need to get user info first
  
  /* -------------------------------- users -------------------------------- */
  let users_json = match fetch_json["globalObjects"]["users"].as_object() {
    Some(users) => users,
    None => return Ok(Vec::new()),
  };
  let mut user_id_to_name_map: HashMap<&str, &str> = HashMap::new();
  for (_, user_json) in users_json {
    let id = user_json["id_str"].as_str().unwrap();
    let name = user_json["screen_name"].as_str().unwrap();
    user_id_to_name_map.insert(id, name);
  }

  let tweets_json = fetch_json["globalObjects"]["tweets"].as_object().unwrap();

  for (_, tweet_json) in tweets_json {
    let id = tweet_json["id_str"].as_str().unwrap().to_string();
    let user = user_id_to_name_map[tweet_json["user_id_str"].as_str().unwrap()].to_string();
    let text = tweet_json["full_text"].as_str().unwrap().to_string();
    let media = parse_media(tweet_json);
    let urls = parse_urls(tweet_json);
    let thread_id = tweet_json.get("self_thread")
      .and_then(|o| Some(o.get("id_str").unwrap().as_str().unwrap().to_string()));
    let date = tweet_json["created_at"].as_str().unwrap().to_string();
    let quoted_tweet_id = tweet_json.get("quoted_status_id_str")
      .and_then(|o| o.as_str()).and_then(|s| Some(s.to_string()));
    let retweeted_tweet_id = tweet_json.get("retweeted_status_id_str")
      .and_then(|o| o.as_str()).and_then(|s| Some(s.to_string()));
    let faves = tweet_json["favorite_count"].as_u64().unwrap();

    let parsed_tweet = Tweet {
      id: id.clone(),
      user,
      text,
      media,
      urls,
      quote: None,
      thread_id,
      extra: Some(TweetExtra {
        date,
        retweeted_by: None,
        faves,
      }),
    };
    parsed_tweets_map.insert(id, (parsed_tweet, quoted_tweet_id, retweeted_tweet_id));
  }

  // these are all the ids of actual tweets, rather than e.g. quoted tweets.
  // note: the id for a retweet is the retweet item, rather than actual tweet
  let timeline_tweet_ids = &fetch_json["timeline"]["instructions"][0]
    ["addEntries"]["entries"].as_array().unwrap().into_iter()
    .filter_map(|item| {
      let id = item["entryId"].as_str().unwrap();
      match id.starts_with("tweet-") {
        true => Some((&id[6..]).to_string()),
        false => None,
      }
    }).collect::<Vec<String>>();

  let parsed_tweets: Vec<Tweet> = timeline_tweet_ids.into_iter()
    .map(|id| {
      let (mut tweet_item, mut quoted_tweet_id, retweeted_tweet_id,
      ) = parsed_tweets_map.get(id).unwrap().clone();

      /*
      retweets have an item for the tweet and an item for the retweet, though 
      it seems the main difference is that the retweet tweet.text starts with
      "RT @user: ", where user is the user of the tweet, not the retweeter. 
      thus, the retweet is essentially a duplicate, so we can ignore/delete the 
      retweet items

      though, it might be nice to know it is a retweet, so add a retweeted_by 
      property to the tweet
      the retweet has the property retweeted_status_id_str, which is the id of 
      the retweeted tweet, and user_id_str, which is the id of the user that 
      retweeted
      */

      // if this is a retweet item, return the retweeted tweet
      if let Some(retweeted_tweet_id) = retweeted_tweet_id {
        // get who retweeted it
        // FIXME: IF THIS GETS RETWEETED BY TWO PEOPLE, DOES IT FUCK UP BC THIS 
        // IMPLEMENTATION DOESN'T ALLOW YOU TO ADD A USER IF THERE IS AN 
        // EXISTING USER??? THEN AGAIN, DO RETWEET ITEMS GET COMBINED INTO 
        // ONE??? HOW DO I GET BOTH USERS FROM THE RETWEET ITEM??
        let retweeted_by = tweet_item.user.clone();

        // swap the tweet to the retweeted tweet, then add who it was retweeted 
        // by (we are changing the original tweet_item/quoted_tweet_id so we 
        // can process add the quoted tweet with the same code as w/ 
        // non-retweeted tweet)
        (tweet_item, quoted_tweet_id, _,) = parsed_tweets_map
          .get(&retweeted_tweet_id).unwrap().clone();
        tweet_item.extra.as_mut().unwrap().retweeted_by = Some(vec![retweeted_by]);
      }

      /*
      quote tweet items do not contain their quoted tweet, instead the quoted 
      tweet is its own item. thus, we must manually assign quoted tweets to 
      their quote tweet
      */

      // if this tweet quotes a tweet, add the quoted tweet to it
      if let Some(quoted_tweet_id) = quoted_tweet_id {
        let (q_tweet_item, _, _,) = parsed_tweets_map
          .get(&quoted_tweet_id).unwrap().clone();
        tweet_item.quote = Some(Box::new(q_tweet_item.clone()));
      }
      tweet_item
    }).collect();

  Ok(parsed_tweets)
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
