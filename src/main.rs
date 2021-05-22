use std::path::PathBuf;
use std::process::exit;

use config::{Config, File};
use log::debug;
use structopt::StructOpt;

use program_guide_tvmaze_api::program_guide::{Database, Episode, Program};
use program_guide_tvmaze_api::tvmaze::TvMazeApi;

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str), long = "config", short = "c")]
    config: PathBuf,
    #[structopt(long = "tvmaze-id", short = "i")]
    tvmaze_id: Option<u32>,
}

static TVMAZE_API_URL_KEY: &str = "tvmaze_api_url";
static DATABASE_URL_KEY: &str = "database_url";

fn main() {
    env_logger::init();

    let args = Cli::from_args();

    let mut config = Config::default();
    if let Err(error) = config.merge(File::from(args.config)) {
        eprintln!("Error reading config: {}", error);
        exit(exitcode::CONFIG);
    }

    let tvmaze_api_url = config.get_str(TVMAZE_API_URL_KEY).ok();
    if tvmaze_api_url == None {
        eprintln!("{} is missing from the config", TVMAZE_API_URL_KEY);
        exit(exitcode::CONFIG);
    }
    let database_url = config.get_str(DATABASE_URL_KEY).ok();
    if database_url == None {
        eprintln!("{} is mising from the config", DATABASE_URL_KEY);
        exit(exitcode::CONFIG);
    }

    debug!("database_url: {:?}", database_url);
    let mut database = match Database::new(database_url.unwrap()) {
        Ok(d) => d,
        Err(error) => {
            eprintln!("Error initializing database: {}", error);
            exit(exitcode::UNAVAILABLE);
        }
    };

    debug!("tvmaze_api_url: {:?}", tvmaze_api_url);
    let tvmaze_api = TvMazeApi::new(tvmaze_api_url.unwrap());

    let mut programs_to_update = Vec::new();

    // If an id was specified, then we only want to update that one.
    if args.tvmaze_id.is_some() {
        let tvmaze_id = args.tvmaze_id.unwrap();
        println!("Updating program for tvmaze_id {}", tvmaze_id);
        debug!("Updating program for tvmaze_id {}", tvmaze_id);

        let program = match database.get_program_by_tvmaze_id(tvmaze_id) {
            Ok(p) => p,
            Err(error) => {
                eprintln!(
                    "Error getting program for tvmaze_id {} from the database: {}",
                    tvmaze_id, error
                );
                exit(exitcode::SOFTWARE);
            }
        };
        debug!("Got program {:?}", program);

        if let Some(program) = program {
            programs_to_update.push(program);
        } else {
            println!(
                "The program for tvmaze_id {} doesn't exist in the database",
                tvmaze_id
            );
            debug!(
                "The program for tvmaze_id {} doesn't exist in the database",
                tvmaze_id
            );
            exit(exitcode::OK);
        }
    }
    // An id was not specified, so potentially update all of the programs in the database.
    else {
        println!("Updating all programs");
        debug!("Updating all programs");

        let mut all_programs = match database.get_all_programs_to_update() {
            Ok(list) => list,
            Err(error) => {
                eprintln!("Error getting all programs from the database: {}", error);
                exit(exitcode::SOFTWARE);
            }
        };
        debug!("Got {} programs from the database", all_programs.len());

        // Consult the api and determine if there are any changes based on the last update date.
        let updates = match tvmaze_api.get_show_updates() {
            Ok(u) => u,
            Err(error) => {
                eprintln!("Error getting show updates from the tvmaze api: {}", error);
                exit(exitcode::SOFTWARE);
            }
        };

        if let Some(updates) = updates {
            debug!("Got {} update dates from the tvmaze api", updates.len());

            debug!("Comparing update dates");
            for program in all_programs {
                // Get the last update from the api, if it exists.
                let last_tvmaze_update: u32 = match updates.get(&program.tvmaze_id) {
                    Some(last_update) => *last_update,
                    None => 0,
                };

                // Get the last update from the database, if it exists.
                let last_program_guide_update: u32 = program.last_update.unwrap_or(0);

                debug!(
                    "last_tvmaze_update: {} last_program_guide_update: {}",
                    last_tvmaze_update, last_program_guide_update
                );
                if last_tvmaze_update != last_program_guide_update {
                    debug!("Adding {} to the list to update", program.name);

                    programs_to_update.push(program);
                }
            }
        } else {
            debug!(
                "Did not get the show update dates from the tvmaze api, so updating all programs"
            );
            programs_to_update.append(all_programs.as_mut());
        }
    }

    for program in programs_to_update {
        println!("Updating {}", program.name);
        debug!("Updating {}", program.name);

        let show = match tvmaze_api.get_show(program.tvmaze_id) {
            Ok(s) => s,
            Err(error) => {
                eprintln!(
                    "Error getting show for tvmaze_id {} from the tvmaze api: {}",
                    program.tvmaze_id, error
                );
                exit(exitcode::SOFTWARE);
            }
        };
        if show.is_none() {
            println!(
                "Did not get a show from the tvmaze api for tvmaze_id {}, so skipping",
                program.tvmaze_id
            );
            debug!(
                "Did not get a show from the tvmaze api for tvmaze_id {}, so skipping",
                program.tvmaze_id
            );
            continue;
        }

        let show = show.unwrap();
        let network = match show.network {
            Some(network) => network.name,
            None => match show.web_channel {
                Some(web) => web.name,
                None => "".to_string(),
            },
        };

        let program_to_update = Program {
            id: program.id,
            name: show.name,
            url: show.url,
            do_update: program.do_update,
            tvmaze_id: program.tvmaze_id,
            network: Some(network),
            last_update: Some(show.updated),
        };

        if program != program_to_update {
            println!("program before: {:?}", program);
            println!("program after:  {:?}", program_to_update);
            debug!("program before: {:?}", program);
            debug!("program after:  {:?}", program_to_update);
        }

        let update_count = match database.update_program(program_to_update) {
            Ok(c) => c,
            Err(error) => {
                eprintln!("Error updating program: {}", error);
                0
            }
        };
        debug!("Update program affected rows: {}", update_count);

        let mut episodes_to_insert: Vec<Episode> = Vec::new();
        let episodes = match tvmaze_api.get_episodes(program.tvmaze_id) {
            Ok(e) => e,
            Err(error) => {
                eprintln!(
                    "Error getting episodes for tvmaze_id {} from the tvmaze api: {}",
                    program.tvmaze_id, error
                );
                exit(exitcode::SOFTWARE);
            }
        };
        if episodes.is_none() {
            println!(
                "Did not get any episodes for tvmaze_id {} from the tvmaze api, so skipping",
                program.tvmaze_id
            );
            continue;
        }

        let episodes = episodes.unwrap();

        for episode in episodes {
            let current_episode = match database.get_episode_by_episode_number(
                program.id,
                episode.season,
                episode.number,
            ) {
                Ok(e) => e,
                Err(error) => {
                    eprintln!("Error getting episode for id {}, season {}, number {} from the database: {}", program.id, episode.season, episode.number, error);
                    None
                }
            };

            let new_episode = Episode {
                program_id: program.id,
                season: episode.season,
                number: episode.number,
                original_air_date: episode.airdate,
                title: episode.name,
                summary_url: episode.url,
            };

            if let Some(current_episode) = current_episode {
                if current_episode != new_episode {
                    println!("episode before: {:?}", current_episode);
                    println!("episode after:  {:?}", new_episode);
                    debug!("episode before: {:?}", current_episode);
                    debug!("episode after:  {:?}", new_episode);
                }
            } else {
                println!("New: {:?}", new_episode);
                debug!("New: {:?}", new_episode);
            }

            episodes_to_insert.push(new_episode);
        }

        let delete_result = match database.delete_episodes_by_program_id(program.id) {
            Ok(count) => (count, true),
            Err(error) => {
                eprintln!("Error deleting episodes for id {}: {}", program.id, error);
                (0, false)
            }
        };
        debug!("Delete episodes affected rows: {}", delete_result.0);

        if delete_result.1 {
            let insert_count = match database.insert_episodes_by_program_id(episodes_to_insert) {
                Ok(count) => count,
                Err(error) => {
                    eprintln!("Error inserting episodes for id {}: {}", program.id, error);
                    0
                }
            };
            debug!("Insert episodes affected rows: {}", insert_count);
            println!(
                "Episode count went from {} to {}",
                delete_result.0, insert_count
            );
        } else {
            debug!("Delete episodes error'd, so skipping episodes insert");
        }
    }

    exit(exitcode::OK);
}
