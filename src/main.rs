extern crate serde_json;

use std::env;
use std::error::Error;

use program_guide_tvmaze_api::program_guide;
use program_guide_tvmaze_api::tvmaze;

// https://www.tvmaze.com/api
fn main() -> std::result::Result<(), Box<dyn Error>> {
    let url = "mysql://pguser:pgus3r@localhost:3306/program_guide";
    let mut database = program_guide::Database::init(&url)?;
    let tvmaze = tvmaze::TVMaze::init();

    let args: Vec<String> = env::args().collect();
    let tvmaze_id = match args.get(1) {
        Some(id) => {
            match id.parse::<u32>() {
                Ok(id) => id,
                Err(_err) => 0
            }
        },
        None => 0
    };

    // If an id was specified, only update that one, else attempt to update all.
    let programs = match tvmaze_id {
        0 => {
            println!("Getting all programs from the database...");
            database.get_all_programs()?
        }
        _ => {
            println!("Updating program for tvmaze_id {}", tvmaze_id);
            database.get_program_by_tvmaze_id(tvmaze_id)?
        }
    };

    // todo Not necessary if updating a single show.
    println!("Getting show update dates...");
    let statuses = tvmaze.get_show_status();

    for program in programs {
        println!("Processing {}", program.name);

        let last_tvmaze_update: u32 = match statuses.get(&program.tvmaze_id) {
            Some(last_update) => *last_update,
            None => 0
        };

        let last_program_guide_update: u32 = match program.last_update {
            Some(last_update) => last_update,
            None => 0
        };

        println!("    tvmaze_update: {} program_guide_update: {}", last_tvmaze_update, last_program_guide_update);
        if last_tvmaze_update == last_program_guide_update {
            println!("    nothing has changed since last run, skipping");
            continue;
        }

        println!("    getting show info from tvmaze...");
        let show = tvmaze.get_show(program.tvmaze_id);

        let network = match show.network {
            Some(network) => network.name,
            None => {
                match show.web_channel {
                    Some(web) => web.name,
                    None => String::from("")
                }
            }
        };

        let updated_program = program_guide::Program {
            id: program.id,
            name: show.name,
            url: show.url,
            do_update: program.do_update,
            tvmaze_id: program.tvmaze_id,
            network: Some(network),
            last_update: Some(last_tvmaze_update),
        };

        println!("    before: {:?}", program);
        println!("    after:  {:?}", updated_program);

        println!("    updating database...");
        database.update_program(updated_program)?;

        println!("    getting episodes from tvmaze...");
        let mut episodes_to_insert: Vec<program_guide::Episode> = Vec::new();
        let episodes: Vec<tvmaze::Episode> = tvmaze.get_episodes(program.tvmaze_id);

        for episode in episodes {
            let new_episode = program_guide::Episode {
                program_id: program.id,
                season: episode.season,
                number: episode.number,
                original_air_date: episode.airdate,
                title: episode.name,
                summary_url: episode.url,
            };
            episodes_to_insert.push(new_episode);
        }

        // todo should be in a transaction

        // todo return the row count
        println!("    deleting current episodes from database...");
        database.delete_episodes_by_program_id(program.id)?;

        println!("    inserting updated episodes into database...");
        database.insert_episodes_by_program_id(episodes_to_insert)?;
    }

    Ok(())
}
