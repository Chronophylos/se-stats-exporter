use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not build http client")]
    BuildClientError(#[source] reqwest::Error),

    #[error("Could not send {method} reqwest to {url}")]
    SendRequestError {
        method: &'static str,
        url: String,
        source: reqwest::Error,
    },

    #[error("Could not parse json")]
    ParseJsonError(#[source] reqwest::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Channel<'a> {
    pub channel: Cow<'a, str>,
    pub messages: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatEmotes<'a> {
    pub username: Cow<'a, str>,
    pub emotes: EmoteList<'a>,
    #[serde(rename = "lastMessage")]
    pub last_message: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmoteList<'a> {
    #[serde(rename = "bttvGlobalEmotes")]
    pub bttv_global_emotes: HashMap<Cow<'a, str>, Emote<'a>>,
    #[serde(rename = "bttvChannelEmotes")]
    pub bttv_channel_emotes: HashMap<Cow<'a, str>, Emote<'a>>,
    #[serde(rename = "ffzGlobalEmotes")]
    pub ffz_global_emotes: HashMap<Cow<'a, str>, Emote<'a>>,
    #[serde(rename = "ffzChannelEmotes")]
    pub ffz_channel_emotes: HashMap<Cow<'a, str>, Emote<'a>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Emote<'a> {
    pub name: Cow<'a, str>,
    #[serde(rename = "_id")]
    pub id: Cow<'a, str>,
    #[serde(rename = "type")]
    pub typ: EmoteType,
    pub width: u8,
    pub height: u8,
    pub gif: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename = "lowercase")]
pub enum EmoteType {
    BTTV,
    FFZ,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatStats<'a> {
    pub channel: Cow<'a, str>,
    #[serde(rename = "totalMessages")]
    pub total_messages: u64,
    pub chatters: Cow<'a, [ChatterStats<'a>]>,
    pub hashtags: Cow<'a, [HashtagStats<'a>]>,
    pub commands: Cow<'a, [CommandStats<'a>]>,
    #[serde(rename = "bttvEmotes")]
    pub bttv_emotes: Cow<'a, [EmoteStats<'a>]>,
    #[serde(rename = "ffzEmotes")]
    pub ffz_emotes: Cow<'a, [EmoteStats<'a>]>,
    #[serde(rename = "twitchEmotes")]
    pub twitch_emotes: Cow<'a, [EmoteStats<'a>]>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatterStats<'a> {
    pub name: Cow<'a, str>,
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HashtagStats<'a> {
    pub hashtag: Cow<'a, str>,
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandStats<'a> {
    pub command: Cow<'a, str>,
    pub amount: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmoteStats<'a> {
    pub id: Cow<'a, str>,
    pub emote: Cow<'a, str>,
    pub amount: u64,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
}

impl ApiClient {
    pub fn new() -> Result<ApiClient, Error> {
        let client = reqwest::ClientBuilder::new()
            .build()
            .map_err(|e| Error::BuildClientError(e))?;

        Ok(ApiClient { client })
    }

    pub async fn get_top_channels<'a>(&self) -> Result<Cow<'a, [Channel<'a>]>, Error> {
        const URL: &str = "https://api.streamelements.com/kappa/v2/chatstats";

        let channels = self
            .client
            .get(URL)
            .send()
            .await
            .map_err(|source| Error::SendRequestError {
                method: "GET",
                url: URL.to_string(),
                source,
            })?
            .json()
            .await
            .map_err(|e| Error::ParseJsonError(e))?;

        Ok(channels)
    }

    pub async fn get_stats<'a, S>(&self, channel: S) -> Result<ChatStats<'a>, Error>
    where
        S: AsRef<str>,
    {
        let url = format!(
            "https://api.streamelements.com/kappa/v2/chatstats/{}/stats",
            channel.as_ref()
        );

        let stats = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|source| Error::SendRequestError {
                method: "GET",
                url: url.clone(),
                source,
            })?
            .json()
            .await
            .map_err(|e| Error::ParseJsonError(e))?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiClient, Error};

    #[tokio::test]
    async fn get_top_channels() -> Result<(), Error> {
        let client = ApiClient::new()?;
        let channels = client.get_top_channels().await?;

        assert_eq!(channels.len(), 100);

        Ok(())
    }

    #[tokio::test]
    async fn get_global_stats() -> Result<(), Error> {
        let client = ApiClient::new()?;
        let stats = client.get_stats("global").await?;

        assert_eq!(stats.channel, "global");
        assert!(stats.total_messages > 67397996744);
        assert_eq!(stats.commands.len(), 100);
        assert_eq!(stats.hashtags.len(), 100);
        assert_eq!(stats.bttv_emotes.len(), 100);
        assert_eq!(stats.ffz_emotes.len(), 100);
        assert_eq!(stats.twitch_emotes.len(), 100);

        Ok(())
    }

    #[test]
    fn sanity_check_message_count_fits_in_u64() {
        let _: u64 = 67397996744;
    }
}
