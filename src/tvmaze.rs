extern crate reqwest;

use serde::Deserialize;
use std::collections::HashMap;

// #[derive(Deserialize)]
// pub struct Status {
//     show: Map<u32, u32>
// }

#[derive(Deserialize)]
pub struct WebChannel {
    pub name: String
}

#[derive(Deserialize)]
pub struct Network {
    pub name: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub network: Option<Network>,
    pub web_channel: Option<WebChannel>
}

#[derive(Deserialize)]
pub struct Episode {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub season: u8,
    pub number: u8,
    pub airdate: String,
}

pub struct TVMaze {
    //client: reqwest::Client
}

impl TVMaze {
    pub fn init() -> TVMaze {
        // let client = reqwest::Client::new();
        let tvmaze = TVMaze {
            // client
        };
        tvmaze
    }

    pub fn get_show_status(&self) -> HashMap<u32, u32> {
        let response = reqwest::blocking::get("http://api.tvmaze.com/updates/shows").expect("todo fixme");
        let text = response.text().expect("todo fixme");
        let status: HashMap<u32, u32> = serde_json::from_str(text.as_str()).unwrap();
        status
    }

    pub fn get_show(&self, id: u32) -> Show {
        let mut url = String::from("http://api.tvmaze.com/shows/");
        url.push_str(id.to_string().as_str());
        let response = reqwest::blocking::get(&url).expect("todo fixme");
        let text = response.text().expect("todo fixme");
        let show: Show = serde_json::from_str(text.as_str()).unwrap();
        show
    }

    pub fn get_episodes(&self, id: u32) -> Vec<Episode> {
        let mut url = String::from("http://api.tvmaze.com/shows/");
        url.push_str(id.to_string().as_str());
        url.push_str("/episodes");
        let response = reqwest::blocking::get(&url).expect("todo fixme");
        let text = response.text().expect("todo fixme");
        let episodes: Vec<Episode> = serde_json::from_str(text.as_str()).unwrap();
        episodes
    }
}