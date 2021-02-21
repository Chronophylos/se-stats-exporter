use flume::{Receiver, Sender};
use serde::Deserialize;
use std::borrow::Cow;
use tokio::task::JoinHandle;
use tungstenite::Message;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not connect to websocket")]
    ConnectToServerError(#[source] tungstenite::Error),

    #[error("Could not receive message meant to be send to websocket")]
    RecvOutgoingMessageError(#[source] flume::RecvError),

    #[error("Could not send message meant to be send to websocket")]
    SendOutgoingMessageError(#[source] flume::SendError<Message>),

    #[error("Could not recv message read from websocket")]
    RecvIncomingMessageError(#[source] flume::RecvError),

    #[error("Could not send message read from websocket")]
    SendIncomingMessageError(#[source] flume::SendError<Message>),

    #[error("Could not read message from websocket")]
    ReadMessageError(#[source] tungstenite::Error),

    #[error("Could not write message to websocket")]
    WriteMessageError(#[source] tungstenite::Error),

    #[error("Could not join task handle")]
    JoinHandleError(#[source] tokio::task::JoinError),

    #[error("Could not convert websocket message to string")]
    ConvertWsMessage(#[source] tungstenite::Error),

    #[error("Could not parse websocket message to json")]
    ParseMessageError(#[source] serde_json::Error),
}

// {
//   "id": "3d40e110-24fe-48a2-a76f-eac2b380ddb3",
//   "type": "message",
//   "destination": {
//     "type": "room",
//     "value": "twitchstats:fischklatscher:stats"
//   },
//   "event": "",
//   "data": [
//     {
//       "type": "chatters",
//       "key": "fishpat",
//       "amount": 1
//     },
//     {
//       "type": "emotes",
//       "key": "DuckerZ",
//       "id": "573d38b50ffbf6cc5cc38dc9",
//       "provider": "bttv",
//       "amount": 62
//     }
//   ]
// }
// TODO: event can be batch instead, then its a list of lists of changes
#[derive(Debug, Clone, Deserialize)]
pub struct RawStatsMessage<'a> {
    id: Cow<'a, str>,
    #[serde(rename = "type")]
    typ: Cow<'a, str>,
    data: Cow<'a, [StatsChangeMessage<'a>]>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StatsChangeMessage<'a> {
    Chatters {
        key: Cow<'a, str>,
        amount: u64,
    },
    Emotes {
        key: Cow<'a, str>,
        id: Cow<'a, str>,
        provider: Cow<'a, str>,
        amount: u64,
    },
}

pub struct WsClient {
    incoming: Receiver<Message>,
    outgoing: Sender<Message>,
    join_handle: JoinHandle<Result<(), Error>>,
}

impl WsClient {
    pub fn new() -> Result<Self, Error> {
        let (mut ws, _resp) = tungstenite::connect("wss://twitchstats-ws.streamelements.com")
            .map_err(|e| Error::ConnectToServerError(e))?;

        let (outgoing_message_sender, outgoing_message_receiver) = flume::bounded(32);
        let (incoming_message_sender, incoming_message_receiver) = flume::bounded(1024);

        let join_handle = tokio::spawn(async move {
            while ws.can_write()
                && ws.can_read()
                && !incoming_message_sender.is_disconnected()
                && !outgoing_message_receiver.is_disconnected()
            {
                if !outgoing_message_receiver.is_empty() {
                    let message = outgoing_message_receiver
                        .recv()
                        .map_err(|e| Error::RecvOutgoingMessageError(e))?;

                    ws.write_message(message)
                        .map_err(|e| Error::WriteMessageError(e))?;
                }

                if !incoming_message_sender.is_full() {
                    let message = ws.read_message().map_err(|e| Error::ReadMessageError(e))?;

                    incoming_message_sender
                        .send(message)
                        .map_err(|e| Error::SendIncomingMessageError(e))?;
                }
            }

            Ok(())
        });

        Ok(Self {
            incoming: incoming_message_receiver,
            outgoing: outgoing_message_sender,
            join_handle,
        })
    }

    pub async fn subscribe_to_stats<S>(&self, channel: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        self.outgoing
            .send_async(Message::Text(format!(
                r#"{{"command":"subscribe","data":{{"room":"twitchstats:{}:stats"}}}}"#,
                channel.as_ref()
            )))
            .await
            .map_err(|e| Error::SendOutgoingMessageError(e))
    }

    pub async fn recv_message(&self) -> Result<Vec<StatsChangeMessage<'_>>, Error> {
        let ws_message = self
            .incoming
            .recv_async()
            .await
            .map_err(|e| Error::RecvIncomingMessageError(e))?
            .into_text()
            .map_err(|e| Error::ConvertWsMessage(e))?;

        let message = serde_json::from_str::<RawStatsMessage>(&ws_message)
            .map_err(|e| Error::ParseMessageError(e))?;

        Ok(message.data.into_owned())
    }

    pub async fn join(self) -> Result<(), Error> {
        match self.join_handle.await {
            Ok(r) => r,
            Err(e) => Err(Error::JoinHandleError(e)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::RawStatsMessage;

    #[test]
    fn stat_message_parsing1() {
        let json = r#"
            {
              "id": "3d40e110-24fe-48a2-a76f-eac2b380ddb3",
              "type": "message",
              "destination": {
                "type": "room",
                "value": "twitchstats:fischklatscher:stats"
              },
              "event": "",
              "data": [
                {
                  "type": "chatters",
                  "key": "fishpat",
                  "amount": 1
                },
                {
                  "type": "emotes",
                  "key": "DuckerZ",
                  "id": "573d38b50ffbf6cc5cc38dc9",
                  "provider": "bttv",
                  "amount": 62
                }
              ]
            }
        "#;

        let message: RawStatsMessage = serde_json::from_str(json).unwrap();

        assert_eq!(message.id, "3d40e110-24fe-48a2-a76f-eac2b380ddb3");
        assert_eq!(message.data.len(), 2);
    }

    #[test]
    fn stat_message_parsing2() {
        let json = r#"{"id":"93fcff69-eac2-42a3-89a7-077e9ca07cb0","type":"message","destination":{"type":"room","value":"twitchstats:global:stats"},"event":"batch","data":[[{"type":"chatters","key":"bnobrabo","amount":1}],[{"type":"chatters","key":"frequency__","amount":1},{"type":"emotes","key":"pgsmTop","id":"304753143","provider":"twitch","amount":1},{"type":"emotes","key":"pgsmCool","id":"304825186","provider":"twitch","amount":1}],[{"type":"chatters","key":"me_myself1","amount":1}],[{"type":"chatters","key":"line171","amount":1},{"type":"emotes","key":"KEKW","id":"5ff647e635fd7d2fe19a5d42","provider":"bttv","amount":1}],[{"type":"chatters","key":"samsone","amount":1},{"type":"emotes","key":"LUL","id":"425618","provider":"twitch","amount":1}],[{"type":"chatters","key":"haslo98","amount":1}],[{"type":"chatters","key":"cosmos__","amount":1}],[{"type":"chatters","key":"iothman99","amount":1},{"type":"emotes","key":"s8hadiSalwa","id":"304826453","provider":"twitch","amount":1}],[{"type":"chatters","key":"nikifores","amount":1}],[{"type":"chatters","key":"nobaqui","amount":1}],[{"type":"chatters","key":"namisocute","amount":1}]]}"#;

        let message: RawStatsMessage = serde_json::from_str(json).unwrap();

        assert_eq!(message.id, "93fcff69-eac2-42a3-89a7-077e9ca07cb0")
    }
}
