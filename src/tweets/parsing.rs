use serde_json::Value;
use crate::types::{TweetURLs, TweetMedia};

pub fn parse_urls(json: &Value) -> Option<Vec<TweetURLs>> {
  match json["entities"]["urls"].as_array() {
    Some(urls_json) => {
      let mut urls: Vec<TweetURLs> = Vec::new();
      for url_json in urls_json {
          let item = TweetURLs {
              shortened_url: url_json["url"].as_str().unwrap().to_string(),
              full_url: url_json["expanded_url"].as_str().unwrap().to_string(),
          };
          urls.push(item);
      }
      Some(urls)
    },
    None => None,
  }
}

pub fn parse_media(json: &Value) -> Option<Vec<TweetMedia>> {
  if let Some(media_json) = json["extended_entities"]["media"].as_array() {
    let mut media: Vec<TweetMedia> = Vec::new();

    for item in media_json {
      let shortened_img_url = item["url"].as_str().unwrap().to_string();
      let full_img_url = item["media_url_https"].as_str().unwrap().to_string();
      let kind = item["type"].as_str().unwrap().to_string(); // i.e. photo or video

      let mut video_url: Option<String> = None;
      if kind == "video" {
        // sort array by bitrate so that the highest bitrate variant is first in 
        // the array. the .m3u8` variant doesn't have a bitrate property, so must 
        // use `?? -1` to push it to the end of the array
        let variants = item["video_info"]["variants"].as_array().unwrap();
        let mut highest_bitrate = 0;
        let mut highest_bitrate_mp4_url = "";
        for variant in variants {
          // need default val bc one of the variants never has a bitrate val
          let bitrate = variant["bitrate"].as_i64().unwrap_or(0);
          if bitrate > highest_bitrate {
            highest_bitrate = bitrate;
            highest_bitrate_mp4_url = variant["url"].as_str().unwrap();
          }
        }
        video_url = Some(highest_bitrate_mp4_url.to_string());
      } else if kind == "animated_gif" {
        // only one entry in the variants array for gifs
        video_url = Some(
          item["video_info"]["variants"][0]["url"].as_str().unwrap().to_string()
        );
      }
      let media_item = TweetMedia {
        shortened_img_url,
        full_img_url,
        kind,
        video_url,
      };
      media.push(media_item);
    }
    Some(media)
  } else {
    None
  }
}
