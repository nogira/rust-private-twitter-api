use super::url::{url_to_tweets, url_to_recommended_tweets};


/* ---------------------------- text only tweets ---------------------------- */

#[tokio::test]
async fn url_test_text_only_tweets_1() {
  println!("url_to_tweets()  //  thread, 1st-tweet");
  let url = "https://twitter.com/epolynya/status/1513868637307691009";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_2() {
  println!("url_to_tweets()  //  thread, mid-tweet");
  let url = "https://twitter.com/epolynya/status/1513868642974244866";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_3() {
  println!("url_to_tweets()  //  thread, last-tweet");
  let url = "https://twitter.com/epolynya/status/1513376048594882560";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_4() {
  println!("url_to_tweets()  //  not thread");
  let url = "https://twitter.com/epolynya/status/1515896927828672514";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_5() {
  println!("url_to_tweets()  //  reply, not thread, items after reply");
  let url = "https://twitter.com/OngoingStudy/status/1515926538662862850";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_6() {
  println!("url_to_tweets()  //  reply, not thread, no items after reply");
  let url = "https://twitter.com/ForbiddenSec/status/1514247615159975940";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn url_test_text_only_tweets_7() {
  println!("url_to_tweets()  //  reply, thread, 1st-tweet");
  let url = "https://twitter.com/epolynya/status/1514815632511963144";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}

// can't find yet
// #[tokio::test]
// async fn url_test_text_only_tweets_8() {
//   println!("url_to_tweets()  //  reply, thread, mid-tweet");
//   let url = "";
//   let tweets = url_to_tweets(url).await;
//   // println!("{:?}", tweets);
//   assert_eq!(tweets.len(), 1)
// }

#[tokio::test]
async fn url_test_text_only_tweets_9() {
  println!("url_to_tweets()  //  reply, thread, last-tweet");
  let url = "https://twitter.com/epolynya/status/1514816123677540355";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

/* quote tweet */

#[tokio::test]
async fn url_test_text_only_tweets_10() {
  println!("url_to_tweets()  //  first tweet, thread, quote tweet, quoted tweet has image");
  let url = "https://twitter.com/balajis/status/1505385989191073793";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}

/* tweet thread so long it has a show more (if this one dies, just search "megathread") */

#[tokio::test]
async fn url_test_text_only_tweets_11() {
  println!("url_to_tweets()  //  thread, 1st-tweet, only 1 show more");
  let url = "https://twitter.com/art_science_x/status/1493630096648949760";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  println!("LENGTH: {:?}", tweets.len());
  println!("LAST TWEET: {:?}", tweets[tweets.len() - 1]);
  assert_eq!(tweets.len(), 35);
}

#[tokio::test]
async fn url_test_text_only_tweets_12() {
  println!("url_to_tweets()  //  thread, 1st-tweet, MULTIPLE show mores");
  let url = "https://twitter.com/mold_time/status/1412827749828513800";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  println!("LENGTH: {:?}", tweets.len());
  println!("LAST TWEET: {:?}", tweets[tweets.len() - 1]);
  assert!(tweets.len() >= 140);
  // i counted the tweets on twitter by hand and 140 is the correct number :)
}

#[tokio::test]
async fn url_test_text_only_tweets_13() {
  println!("url_to_tweets()  //  not thread, 1st-tweet, first reply is \"You’re unable to view this Tweet because this account owner limits who can view their Tweets\"");
  let url = "https://twitter.com/_dalten/status/1556728285840564224";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  // println!("LENGTH: {:?}", tweets.len());
  // println!("LAST TWEET: {:?}", tweets[tweets.len() - 1]);
  // panic!("");
  assert!(tweets.len() == 1);
}

#[tokio::test]
async fn url_test_text_only_tweets_14() {
  println!("url_to_tweets()  //  thread, 1st-tweet, 9th tweet in thread quotes a deleted tweet");
  let url = "https://www.twitter.com/Replit/status/1385430878231224321";
  let tweets = url_to_tweets(url).await;
  assert_eq!(tweets.len(), 11);
}

#[tokio::test]
async fn url_test_text_only_tweets_15() {
  println!("url_to_tweets()  //  thread, 1st-tweet, fails bc of some weird json nesting shit specific to this tweet for some reason");
  let url = "https://www.twitter.com/EthicalSkeptic/status/1588457981645303808";
  let tweets = url_to_tweets(url).await;
  assert_eq!(tweets.len(), 6);
}

#[tokio::test]
async fn url_test_text_only_tweets_16() {
  println!("url_to_tweets()  //  thread, 1st-tweet, fails bc \"TweetWithVisibilityResults\" in quote");
  let url = "https://www.twitter.com/balajis/status/1591040299056660480";
  let tweets = url_to_tweets(url).await;
  assert_eq!(tweets.len(), 2);
}

/* -------------------------------recommended------------------------------- */

#[tokio::test]
async fn url_test_recommended_tweets() {
  println!("url_to_tweets()  //  thread, 1st-tweet");
  let url = "https://twitter.com/epolynya/status/1513868637307691009";
  let tweets = url_to_recommended_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 10); // should be 24, so i prob need to join cursor query
}
