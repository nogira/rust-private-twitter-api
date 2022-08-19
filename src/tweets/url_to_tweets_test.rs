use super::url::url_to_tweets;


/* ---------------------------- text only tweets ---------------------------- */

#[tokio::test]
async fn text_only_tweets_1() {
  println!("url_to_tweets()  //  thread, 1st-tweet");
  let url = "https://twitter.com/epolynya/status/1513868637307691009";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}

#[tokio::test]
async fn text_only_tweets_2() {
  println!("url_to_tweets()  //  thread, mid-tweet");
  let url = "https://twitter.com/epolynya/status/1513868642974244866";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn text_only_tweets_3() {
  println!("url_to_tweets()  //  thread, last-tweet");
  let url = "https://twitter.com/epolynya/status/1513376048594882560";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn text_only_tweets_4() {
  println!("url_to_tweets()  //  not thread");
  let url = "https://twitter.com/epolynya/status/1515896927828672514";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn text_only_tweets_5() {
  println!("url_to_tweets()  //  reply, not thread, items after reply");
  let url = "https://twitter.com/OngoingStudy/status/1515926538662862850";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn text_only_tweets_6() {
  println!("url_to_tweets()  //  reply, not thread, no items after reply");
  let url = "https://twitter.com/ForbiddenSec/status/1514247615159975940";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

#[tokio::test]
async fn text_only_tweets_7() {
  println!("url_to_tweets()  //  reply, thread, 1st-tweet");
  let url = "https://twitter.com/epolynya/status/1514815632511963144";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}

// can't find yet
// #[tokio::test]
// async fn text_only_tweets_8() {
//   println!("url_to_tweets()  //  reply, thread, mid-tweet");
//   let url = "";
//   let tweets = url_to_tweets(url).await;
//   // println!("{:?}", tweets);
//   assert_eq!(tweets.len(), 1)
// }

#[tokio::test]
async fn text_only_tweets_9() {
  println!("url_to_tweets()  //  reply, thread, last-tweet");
  let url = "https://twitter.com/epolynya/status/1514816123677540355";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert_eq!(tweets.len(), 1)
}

/* quote tweet */

#[tokio::test]
async fn t() {
  println!("url_to_tweets()  //  first tweet, thread, quote tweet");
  let url = "https://twitter.com/balajis/status/1505385989191073793";
  let tweets = url_to_tweets(url).await;
  // println!("{:?}", tweets);
  assert!(tweets.len() > 1)
}