use mysql::{Conn, Error, Statement};
use mysql::prelude::Queryable;

#[derive(Debug)]
pub struct Program {
    pub id: u32,
    pub name: String,
    pub url: String,
    pub do_update: bool,
    pub tvmaze_id: u32,
    pub network: Option<String>,
    pub last_update: Option<u32>,
}

impl PartialEq for Program {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id &&
            self.name == other.name &&
            self.url == other.url &&
            self.do_update == other.do_update &&
            self.tvmaze_id == other.tvmaze_id &&
            self.network == other.network
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Episode {
    pub program_id: u32,
    pub season: u8,
    pub number: u8,
    pub original_air_date: String,
    pub title: String,
    pub summary_url: String,
}

pub struct Database {
    connection: Conn,
    get_all_programs: Statement,
    get_program_by_tvmaze_id: Statement,
    update_program_by_tvmaze_id: Statement,
    get_episode_by_episode_number: Statement,
    delete_episodes_by_program_id: Statement,
    insert_episodes_for_program_id: Statement,
}

impl Database {
    pub fn new(url: String) -> Result<Self, Error> {
        let mut connection = Conn::new(url)?;

        let get_all_programs = connection.prep(
            "SELECT id, name, url, do_update, tvmaze_id, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program")?;

        let get_program_by_tvmaze_id = connection.prep(
            "SELECT id, name, url, do_update, tvmaze_id, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program WHERE tvmaze_id = ?")?;

        let update_program_by_tvmaze_id = connection.prep(
            "UPDATE program SET name = ?, url = ?, network = ?, last_update = FROM_UNIXTIME(?) WHERE tvmaze_id = ?")?;

        let get_episode_by_episode_number = connection.prep(
            "SELECT program_id, season, number, IFNULL(DATE_FORMAT(original_air_date, '%Y-%m-%d'), ''), title, summary_url FROM episode WHERE program_id = ? AND season = ? AND number = ?")?;

        let delete_episodes_by_program_id = connection.prep(
            "DELETE FROM episode WHERE program_id = ?")?;

        let insert_episodes_for_program_id = connection.prep(
            "INSERT INTO episode (program_id, season, number, original_air_date, title, serial_number, summary_url) VALUES (?, ?, ?, STR_TO_DATE(?, '%Y-%m-%d'), ?, 0, ?)")?;

        Ok(
            Self {
                connection,
                get_all_programs,
                get_program_by_tvmaze_id,
                update_program_by_tvmaze_id,
                get_episode_by_episode_number,
                delete_episodes_by_program_id,
                insert_episodes_for_program_id,
            }
        )
    }

    pub fn get_program_by_tvmaze_id(&mut self, id: u32) -> Result<Option<Program>, Error> {
        let mut list = self.connection.exec_map(
            &self.get_program_by_tvmaze_id, (&id, ),
            |(id, name, url, do_update, tvmaze_id, network, last_update)| {
                Program { id, name, url, do_update, tvmaze_id, network, last_update }
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
            |(id, name, url, do_update, tvmaze_id, network, last_update)| {
                Program { id, name, url, do_update, tvmaze_id, network, last_update }
            },
        )?;
        Ok(list)
    }

    pub fn update_program(&mut self, program: Program) -> Result<u64, Error> {
        let _result: Vec<u8> = self.connection.exec(&self.update_program_by_tvmaze_id, (&program.name, &program.url, &program.network, &program.last_update, &program.tvmaze_id))?;

        Ok(self.connection.affected_rows())
    }

    pub fn get_episode_by_episode_number(&mut self, id: u32, season: u8, number: u8) -> Result<Option<Episode>, Error> {
        let mut list = self.connection.exec_map(
            &self.get_episode_by_episode_number, (id, season, number),
            |(program_id, season, number, original_air_date, title, summary_url)| {
                Episode { program_id, season, number, original_air_date, title, summary_url }
            },
        )?;

        if list.len() > 0 {
            Ok(list.pop())
        } else {
            Ok(None)
        }
    }

    pub fn delete_episodes_by_program_id(&mut self, id: u32) -> Result<u64, Error> {
        let _result: Vec<u8> = self.connection.exec(&self.delete_episodes_by_program_id, (id, ))?;

        Ok(self.connection.affected_rows())
    }

    pub fn insert_episodes_by_program_id(&mut self, episodes: Vec<Episode>) -> Result<u64, Error> {
        let mut affected_rows = 0;
        for episode in episodes {
            let original_air_date = match episode.original_air_date.trim().is_empty() {
                true => None,
                false => Some(episode.original_air_date)
            };
            let _result: Vec<u8> = self.connection.exec(&self.insert_episodes_for_program_id, (episode.program_id, episode.season, episode.number, original_air_date, episode.title, episode.summary_url))?;
            affected_rows += self.connection.affected_rows();
        }

        Ok(affected_rows)
    }
}