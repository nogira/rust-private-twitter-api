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
  pub quote: Option<Box<Tweet>>,
  /// if the tweet is part of a thread, the id of the first tweet in the thread
  pub thread_id: Option<String>,
  pub extra: Option<TweetExtra>,
}

// FIXME: make Tweet the only tweet struct, then add substructs as optional, e.g.
// also can change `quote` prop to `Option<Box<Tweet>>` to fix error from `Option<Tweet>` 

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

/// additional attributes for `Tweet`, such as `date`, `quoted_tweet_id`, 
/// `retweet_tweet_id`, `retweeted_by`
#[derive(Debug, Clone)]
pub struct TweetExtra {
  pub date: String,
  /// temp attribute to be able match quote tweets to the quoted tweet
  pub quoted_tweet_id: Option<String>,
  pub retweet_tweet_id: Option<String>,
  pub retweeted_by: Option<Vec<String>>,
  pub faves: u64,
}
