use mysql::*;
use mysql::prelude::*;

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

#[derive(Debug)]
pub struct Episode {
    pub program_id: u32,
    pub season: u8,
    pub number: u8,
    pub original_air_date: String,
    pub title: String,
    pub summary_url: String,
}

pub struct Database {
    connection: mysql::Conn,
    get_all_programs: Statement,
    get_program_by_tvmaze_id: Statement,
    update_program_by_tvmaze_id: Statement,
    delete_episodes_by_program_id: Statement,
    insert_episodes_for_program_id: Statement,
}

impl Database {
    pub fn init(url: &str) -> Result<Database> {
        let mut connection = mysql::Conn::new(url)?;
        // todo maybe create these just before each call? or some kind of init_get_all thing? static thing?
        // todo should i be using drop?
        let get_all_programs = connection.prep("SELECT id, name, url, do_update, tvmaze_id, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program")?;
        let get_program_by_tvmaze_id = connection.prep("SELECT id, name, url, do_update, tvmaze_id, network, UNIX_TIMESTAMP(last_update) AS last_update FROM program WHERE tvmaze_id = ?")?;
        let update_program_by_tvmaze_id = connection.prep("UPDATE program SET name = ?, url = ?, network = ?, last_update = FROM_UNIXTIME(?) WHERE tvmaze_id = ?")?;
        let delete_episodes_by_program_id = connection.prep("DELETE FROM episode WHERE program_id = ?")?;
        let insert_episodes_for_program_id = connection.prep("INSERT INTO episode (program_id, season, number, original_air_date, title, serial_number, summary_url) VALUES (?, ?, ?, STR_TO_DATE(?, '%Y-%m-%d'), ?, 0, ?)")?;
        let database = Database {
            connection,
            get_all_programs,
            get_program_by_tvmaze_id,
            update_program_by_tvmaze_id,
            delete_episodes_by_program_id,
            insert_episodes_for_program_id,
        };
        Ok(database)
    }

    // todo use serde
    pub fn get_all_programs(&mut self) -> Result<Vec<Program>> {
        let results = self.connection.exec_map(
            &self.get_all_programs, (),
            |(id, name, url, do_update, tvmaze_id, network, last_update)| {
                Program { id, name, url, do_update, tvmaze_id, network, last_update }
            },
        )?;
        Ok(results)
    }

    pub fn get_program_by_tvmaze_id(&mut self, id: u32) -> Result<Vec<Program>> {
        let results = self.connection.exec_map(
            &self.get_program_by_tvmaze_id, (&id, ),
            |(id, name, url, do_update, tvmaze_id, network, last_update)| {
                Program { id, name, url, do_update, tvmaze_id, network, last_update }
            },
        )?;
        Ok(results)
    }

    pub fn update_program(&mut self, program: Program) -> Result<Vec<u8>> {
        let update_result: Vec<u8> = self.connection.exec(&self.update_program_by_tvmaze_id, (&program.name, &program.url, &program.network, &program.last_update, &program.tvmaze_id))?;
        Ok(update_result)
    }

    pub fn delete_episodes_by_program_id(&mut self, id: u32) -> Result<Vec<u8>> {
        let delete_result: Vec<u8> = self.connection.exec(&self.delete_episodes_by_program_id, (id, ))?;
        Ok(delete_result)
    }

    pub fn insert_episodes_by_program_id(&mut self, episodes: Vec<Episode>) -> Result<Vec<Vec<u8>>> {
        let mut total: Vec<Vec<u8>> = Vec::new();
        for episode in episodes {
            let original_air_date = match episode.original_air_date.trim().is_empty() {
                true => {
                    None
                }
                false => {
                    Some(episode.original_air_date)
                }
            };
            let insert_result: Vec<u8> = self.connection.exec(&self.insert_episodes_for_program_id, (episode.program_id, episode.season, episode.number, original_air_date, episode.title, episode.summary_url))?;
            total.push(insert_result);
        }
        Ok(total)
    }
}