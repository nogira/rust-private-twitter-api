use super::fetch::query_fetch;

#[tokio::test]
async fn fetch_q_test_1() {

  println!("query_fetch()  //  query that gets single quote tweet");
  let query = r#"from:balajis -filter:replies "Emotionally unavailable doesn’t even begin to describe Hayes Rutherford""#;
  let json = query_fetch(&query).await;
//   println!("{:?}", &json);
  panic!("");
//   assert_eq!(tweets.len(), 20);
}

#[tokio::test]
async fn fetch_q_test_2() {

  println!("query_fetch()  //  query that gets a tweet containing a url");
  let query = r#"from:balajis -filter:replies "Why was SBF being protected?""#;
  let json = query_fetch(&query).await;
//   println!("{:?}", &json);
  panic!("");
//   assert_eq!(tweets.len(), 20);
}

