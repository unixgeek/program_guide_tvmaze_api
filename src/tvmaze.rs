use std::collections::HashMap;

use reqwest::{Error, StatusCode};
use reqwest::blocking::Client;
use serde::Deserialize;

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
    pub web_channel: Option<WebChannel>,
    pub updated: u32,
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

pub struct TvMazeApi {
    url: String,
    client: Client,
}

impl TvMazeApi {
    pub fn new(url: String) -> Self {
        let client = Client::new();

        Self {
            url,
            client,
        }
    }

    pub fn get_show_updates(&self) -> Result<Option<HashMap<u32, u32>>, Error> {
        let request = format!("{url}/updates/shows", url = self.url);
        let response = self.client.get(&request).send()?;

        match response.status() {
            StatusCode::OK => Ok(Some(response.json()?)),
            _ => Ok(None)
        }
    }

    pub fn get_show(&self, id: u32) -> Result<Option<Show>, Error> {
        let request = format!("{url}/shows/{id}", url = self.url, id = id);
        let response = self.client.get(&request).send()?;

        match response.status() {
            StatusCode::OK => Ok(Some(response.json()?)),
            _ => Ok(None)
        }
    }

    pub fn get_episodes(&self, id: u32) -> Result<Option<Vec<Episode>>, Error> {
        let request = format!("{url}/shows/{id}/episodes", url = self.url, id = id);
        let response = self.client.get(&request).send()?;

        match response.status() {
            StatusCode::OK => Ok(Some(response.json()?)),
            _ => Ok(None)
        }
    }
}