// TODO: properly implement this v1 twitter api

use serde_json::Value;

const V1_API_TOKEN: &str = "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

pub async fn fetch_tweets_from_user(screen_name: &str, count: u64) -> Value {
  let url = format!("https://api.twitter.com/1.1/statuses/user_timeline.json?screen_name={}&count={}", 
    screen_name, count);

  let client = reqwest::Client::new();
  let req = client.get(url)
    .header("Authorization", V1_API_TOKEN);
  let text = req.try_clone().unwrap()
    .send().await.unwrap()
    .text().await.unwrap();
  let json: Value = serde_json::from_str(&text).unwrap();

  json
}

#[cfg(test)]
#[tokio::test]
async fn test_v1_api() {
  // println!("{:?}", fetch_tweets_from_user("elonmusk", 10).await);
}