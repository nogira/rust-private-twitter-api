use super::query::query_to_tweets;

// #[tokio::test]
// async fn test_1() {
//   println!("query_to_tweets()  //  query that gets tweets");
//   let query = "from:balajis -filter:replies";
//   let tweets = query_to_tweets(&query).await;
//   // println!("{:?}", &tweets);
//   assert_eq!(tweets.len(), 20);
// }

// #[tokio::test]
// async fn test_2() {
//   println!("query_to_tweets()  //  query that gets no tweets");
//   let query = "from:balajis -filter:replies min_faves=999999999";
//   let tweets = query_to_tweets(&query).await;
//   // println!("{:?}", &tweets);
//   assert_eq!(tweets.len(), 0);
// }

// #[tokio::test]
// async fn test_3() {
//   println!("query_to_tweets()  //  query that gets only retweets - testing retweet parsing");
//   let query = "from:elonmusk filter:nativeretweets include:nativeretweets";
//   let tweets = query_to_tweets(&query).await;
//   // println!("{:?}", &tweets);
//   // for some reason only 3 tweets show up from this query. twitter hiding rest of RTs
//   assert!(tweets.len() > 0);
//   // const tweetTexts = tweets.map(x => x.text);
//   // for (const text of tweetTexts) {
//   //     // throws error if finds a retweet
//   //     assert(! text.match(/^RT @.+?: /));
//   // }
// }

// #[tokio::test]
// async fn test_4() {
//   println!("query_to_tweets()  //  testing quote tweets");
//   let query = r#"from:balajis -filter:replies "Emotionally unavailable doesn’t even begin to describe Hayes Rutherford""#;
//   let tweets = query_to_tweets(&query).await;
//   // println!("{:?}", &tweets);
//   assert_eq!(tweets.len(), 1);
// }

// #[tokio::test]
// async fn test_misinfo() {
//   println!("query_to_tweets()  //  testing misinfo filtering");
//   let query = "from:Babygravy9";
//   let tweets = query_to_tweets(&query).await;
//   println!("{:?}", &tweets);
//   println!("MISINFO LEN: {:?}", tweets.len());
//   // assert_eq!(tweets.len(), 1);
// }

#[tokio::test]
async fn test_threads() {
  println!("query_to_tweets()  //  query that gets threads");
  let query = "from:balajis -filter:replies min_faves:500";
  let tweets = query_to_tweets(&query).await;
  println!("{:?}", &tweets[0]);
  println!("NUM TWEETS: {:?}", tweets.len());
  // assert_eq!(tweets.len(), 1);
}