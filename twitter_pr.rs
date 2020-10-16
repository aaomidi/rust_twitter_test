extern crate reqwest;

use std::sync::mpsc::{Receiver, channel};
use serde::{Serialize, Deserialize};
use tokio::stream::StreamExt;
use tokio::io::{AsyncBufReadExt, StreamReader};
use serde_json::{Deserializer, StreamDeserializer};
use std::io::Write;

pub struct TwitterApi {
    pub bearer_api_token: String
}

struct RequestStream {
    receiver: Receiver<bytes::Bytes>
}

impl std::io::Read for RequestStream {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        match self.receiver.recv() {
            Ok(next) => {
                let vec = next.to_vec();
                let vec_len = vec.len();
                println!("Incoming -> {}", String::from_utf8(vec.clone()).unwrap_or("err".to_string()));
                buf.write_all(vec.as_slice());
                return Ok(vec_len);
            }
            Err(_e) => panic!("error!")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Tweet {
    pub text: String,
}

#[derive(Deserialize)]
struct Wrapper<T> {
    data: T,
}

impl TwitterApi {
    pub fn new(api_token: String) -> TwitterApi {
        TwitterApi {
            bearer_api_token: api_token
        }
    }

    pub async fn listen_for_tweets(&self) -> Receiver<Tweet> {
        let (sender, receiver) = channel::<Tweet>();

        let client = reqwest::Client::new();
        let url = reqwest::Url::parse("https://api.twitter.com/2/tweets/search/stream").unwrap();
        let mut req = client.get(url);
        req = req.bearer_auth(&self.bearer_api_token);

        tokio::spawn(async move {
            println!("Requesting...");
            match req.send().await {
                Ok(resp) => {
                    let (stream_sender, stream_receiver) = channel::<bytes::Bytes>();
                    tokio::spawn(async move {
                        let rs = RequestStream {
                            receiver: stream_receiver
                        };

                        let x = Deserializer::from_reader(rs).into_iter::<Wrapper<Tweet>>();
                        for val in x {
                            println!("{:?}", val.unwrap().data);
                        }
                    });

                    let mut stream = resp.bytes_stream();
                    loop {
                        let value = stream.next().await.unwrap();
                        match value {
                            Ok(bytes) => {
                                stream_sender.send(bytes);
                            }
                            Err(err) => {
                                println!("Error! {}", err)
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                }
            };
        });

        receiver
    }
}

