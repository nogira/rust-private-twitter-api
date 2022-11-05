#[derive(Debug, Clone)]
pub struct Tweet {
  /// id number of tweet (last part of url)
  pub id: String,
  /// username of the account who posted the tweet
  pub user: String,
  /// the text of the tweet
  pub text: String,
  pub media: Option<Vec<TweetMedia>>,
  pub urls: Option<Vec<TweetURLs>>,
  pub quote: Option<Quote>,
  /// id of the first tweet in the thread this tweet is in
  pub thread_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TweetMedia {
  /// the twitter shortened url
  pub shortened_img_url: String,
  /// the original image url
  pub full_img_url: String,
  /// `photo` or `video` or `animated_gif`
  pub kind: String,
  pub video_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TweetURLs {
  /// the twitter shortened url
  pub shortened_url: String,
  /// the original url
  pub full_url: String,
}

/* ------------------------------TWEET VARIANTS------------------------------ */

/// same as `Tweet`, but no `quote` attribute
#[derive(Debug, Clone)]
pub struct Quote {
  /// id number of tweet (last part of url)
  pub id: String,
  /// username of the account who posted the tweet
  pub user: String,
  /// the text of the tweet
  pub text: String,
  pub media: Option<Vec<TweetMedia>>,
  pub urls: Option<Vec<TweetURLs>>,
  /// id of the first tweet in the thread this tweet is in
  pub thread_id: Option<String>,
}

/// same as `Tweet`, but additional attributes `date`, `quoted_tweet_id`, 
/// `retweet_tweet_id`, `retweeted_by`
#[derive(Debug, Clone)]
pub struct QueryTweet {
  /// id number of tweet (last part of url)
  pub id: String,
  /// username of the account who posted the tweet
  pub user: String,
  /// the text of the tweet
  pub text: String,
  pub media: Option<Vec<TweetMedia>>,
  pub urls: Option<Vec<TweetURLs>>,
  pub quote: Option<Quote>,
  /// if the tweet is part of a thread, the id of the first tweet in the thread
  pub thread_id: Option<String>,

  // ADDITIONAL ATTRIBUTES

  pub date: String,
  /// temp attribute to be able match quote tweets to the quoted tweet
  pub quoted_tweet_id: Option<String>,
  pub retweet_tweet_id: Option<String>,
  pub retweeted_by: Option<Vec<String>>,

  pub faves: u64,
}