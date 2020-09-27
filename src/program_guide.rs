use mysql::{Conn, Error, Statement};
use mysql::prelude::Queryable;

#[derive(Debug)]
pub struct Program {
    pub tvmaze_show_id: u32,
    pub name: Option<String>,
    pub url: Option<String>,
    pub network: Option<String>,
    pub last_update: Option<u32>,
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.tvmaze_show_id == other.tvmaze_show_id &&
            self.name == other.name &&
            self.url == other.url &&
            self.network == other.network
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Episode {
    pub tvmaze_show_id: u32,
    pub season: u8,
    pub number: u8,
    pub original_air_date: Option<String>,
    pub title: Option<String>,
    pub summary_url: Option<String>,
}

pub struct Database {
    connection: Conn,
    get_all_programs: Statement,
    get_program_by_tvmaze_show_id: Statement,
    update_program_by_tvmaze_show_id: Statement,
    get_episode_by_episode_number: Statement,
    delete_episodes_by_tvmaze_show_id: Statement,
    insert_episodes_for_tvmaze_show_id: Statement,
}

impl Database {
    pub fn new(url: String) -> Result<Self, Error> {
        let mut connection = Conn::new(url)?;

        let get_all_programs = connection.prep(
            "SELECT tvmaze_show_id, name, url, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program")?;

        let get_program_by_tvmaze_show_id = connection.prep(
            "SELECT tvmaze_show_id, name, url, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program WHERE tvmaze_show_id = ?")?;

        let update_program_by_tvmaze_show_id = connection.prep(
            "UPDATE program SET name = ?, url = ?, network = ?, last_update = FROM_UNIXTIME(?) WHERE tvmaze_show_id = ?")?;

        let get_episode_by_episode_number = connection.prep(
            "SELECT tvmaze_show_id, season, number, DATE_FORMAT(original_air_date, '%Y-%m-%d'), title, summary_url FROM episode WHERE tvmaze_show_id = ? AND season = ? AND number = ?")?;

        let delete_episodes_by_tvmaze_show_id = connection.prep(
            "DELETE FROM episode WHERE tvmaze_show_id = ?")?;

        let insert_episodes_for_tvmaze_show_id = connection.prep(
            "INSERT INTO episode (tvmaze_show_id, season, number, original_air_date, title, summary_url) VALUES (?, ?, ?, STR_TO_DATE(?, '%Y-%m-%d'), ?, ?)")?;

        Ok(
            Self {
                connection,
                get_all_programs,
                get_program_by_tvmaze_show_id,
                update_program_by_tvmaze_show_id,
                get_episode_by_episode_number,
                delete_episodes_by_tvmaze_show_id,
                insert_episodes_for_tvmaze_show_id,
            }
        )
    }

    pub fn get_program_by_tvmaze_show_id(&mut self, id: u32) -> Result<Option<Program>, Error> {
        let mut list = self.connection.exec_map(
            &self.get_program_by_tvmaze_show_id, (&id, ),
            |(tvmaze_show_id, name, url, network, last_update)| {
                Program { tvmaze_show_id, name, url, network, last_update }
            },
        )?;

        if list.len() > 0 {
            Ok(list.pop())
        } else {
            Ok(None)
        }
    }

    pub fn get_all_programs(&mut self) -> Result<Vec<Program>, Error> {
        let list = self.connection.exec_map(
            &self.get_all_programs, (),
            |(tvmaze_show_id, name, url, network, last_update)| {
                Program { tvmaze_show_id, name, url, network, last_update }
            },
        )?;
        Ok(list)
    }

    pub fn update_program(&mut self, program: Program) -> Result<u64, Error> {
        let _result: Vec<u8> = self.connection.exec(&self.update_program_by_tvmaze_show_id, (&program.name, &program.url, &program.network, &program.last_update, &program.tvmaze_show_id))?;

        Ok(self.connection.affected_rows())
    }

    pub fn get_episode_by_episode_number(&mut self, id: u32, season: u8, number: u8) -> Result<Option<Episode>, Error> {
        let mut list = self.connection.exec_map(
            &self.get_episode_by_episode_number, (id, season, number),
            |(tvmaze_show_id, season, number, original_air_date, title, summary_url)| {
                Episode { tvmaze_show_id, season, number, original_air_date, title, summary_url }
            },
        )?;

        if list.len() > 0 {
            Ok(list.pop())
        } else {
            Ok(None)
        }
    }

    pub fn delete_episodes_by_program_id(&mut self, id: u32) -> Result<u64, Error> {
        let _result: Vec<u8> = self.connection.exec(&self.delete_episodes_by_tvmaze_show_id, (id, ))?;

        Ok(self.connection.affected_rows())
    }

    pub fn insert_episodes_by_program_id(&mut self, episodes: Vec<Episode>) -> Result<u64, Error> {
        let mut affected_rows = 0;
        for episode in episodes {
            let _result: Vec<u8> = self.connection.exec(&self.insert_episodes_for_tvmaze_show_id, (episode.tvmaze_show_id, episode.season, episode.number, episode.original_air_date, episode.title, episode.summary_url))?;
            affected_rows += self.connection.affected_rows();
        }

        Ok(affected_rows)
    }
}