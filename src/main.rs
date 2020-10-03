use std::path::PathBuf;
use std::process::exit;

use config::{Config, File};
use log::debug;
use structopt::StructOpt;

use program_guide_tvmaze_api::program_guide::{Database, Program};
use program_guide_tvmaze_api::tvmaze::{TvMazeApi, Show};
use program_guide_tvmaze_api::{tvmaze, program_guide};

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str), long = "config", short = "c")]
    config: PathBuf,
    #[structopt(long = "tvmaze-show-id", short = "i")]
    tvmaze_show_id: Option<u32>,
}

static TVMAZE_API_URL_KEY: &'static str = "tvmaze_api_url";
static DATABASE_URL_KEY: &'static str = "database_url";

fn main() {
    env_logger::init();

    let args = Cli::from_args();

    let mut config = Config::default();
    match config.merge(File::from(args.config)) {
        Err(error) => {
            eprintln!("Error reading config: {}", error);
            exit(exitcode::CONFIG);
        }
        _ => ()
    }

    let tvmaze_api_url = config.get_str(TVMAZE_API_URL_KEY).ok();
    if tvmaze_api_url == None {
        eprintln!("{} is missing from the config", TVMAZE_API_URL_KEY);
        exit(exitcode::CONFIG);
    }
    let database_url = config.get_str(DATABASE_URL_KEY).ok();
    if database_url == None {
        eprintln!("{} is missing from the config", DATABASE_URL_KEY);
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
    if args.tvmaze_show_id.is_some() {
        let tvmaze_show_id = args.tvmaze_show_id.unwrap();
        println!("Checking for updates to tvmaze_show_id {}", tvmaze_show_id);
        debug!("Checking for updates to tvmaze_show_id {}", tvmaze_show_id);

        let program = match database.get_program_by_tvmaze_show_id(tvmaze_show_id) {
            Ok(p) => p,
            Err(error) => {
                eprintln!("Error getting program for tvmaze_show_id {} from the database: {}", tvmaze_show_id, error);
                exit(exitcode::SOFTWARE);
            }
        };
        debug!("Got program {:?}", program);

        if program.is_none() {
            println!("The program for tvmaze_show_id {} doesn't exist in the database", tvmaze_show_id);
            debug!("The program for tvmaze_show_id {} doesn't exist in the database", tvmaze_show_id);
            exit(exitcode::OK);
        } else {
            programs_to_update.push(program.unwrap());
        }
    }
    // An id was not specified, so potentially update all of the programs in the database.
    else {
        println!("Checking for updates to all programs");
        debug!("Checking for updates to all programs");

        let mut all_programs = match database.get_all_programs() {
            Ok(list) => list,
            Err(error) => {
                eprintln!("Error getting all programs from the database: {}", error);
                exit(exitcode::SOFTWARE);
            }
        };
        debug!("Got {} programs from the database", all_programs.len());

        /*
         * Consult the api and determine if there are any changes based on the last update date.
         * Even if the show was updated, that doesn't mean there are any changes to the episodes,
         * but we won't know that until we call the episode api.
         */
        let updates = match tvmaze_api.get_show_updates() {
            Ok(u) => u,
            Err(error) => {
                eprintln!("Error getting show updates from the tvmaze api: {}", error);
                exit(exitcode::SOFTWARE);
            }
        };

        if updates.is_some() {
            let updates = updates.unwrap();
            debug!("Got {} update dates from the tvmaze api", updates.len());

            debug!("Comparing update dates");
            for program in all_programs {

                // Get the last update from the api, if it exists.
                let last_tvmaze_update = updates.get(&program.tvmaze_show_id).unwrap_or(&0).clone();

                // Get the last update from the database, if it exists.
                let last_program_guide_update = program.last_update.unwrap_or(0);

                debug!("last_tvmaze_update: {} last_program_guide_update: {}", last_tvmaze_update, last_program_guide_update);
                if last_tvmaze_update != last_program_guide_update {
                    let display = match &program.name {
                        Some(n) => n.clone(),
                        None => format!("tvmaze_show_id {}", program.tvmaze_show_id)
                    };
                    debug!("Adding {} to the list to update", display);

                    programs_to_update.push(program);
                }
            }
        } else {
            debug!("Did not get the show update dates from the tvmaze api, so updating all programs");
            programs_to_update.append(all_programs.as_mut());
        }
    }

    if programs_to_update.is_empty() {
        debug!("Nothing has changed");
        println!("Nothing has changed");
    }

    for program in programs_to_update {
        // Print the name if we have it, otherwise the id.
        let display = match &program.name {
            Some(n) => n.clone(),
            None => format!("tvmaze_show_id {}", program.tvmaze_show_id)
        };

        let mut something_changed = false;

        println!("Checking {}", display);
        debug!("Checking {}", display);

        let show = match tvmaze_api.get_show(program.tvmaze_show_id) {
            Ok(s) => s,
            Err(error) => {
                eprintln!("Error getting show for tvmaze_show_id {} from the tvmaze api: {}", program.tvmaze_show_id, error);
                exit(exitcode::SOFTWARE);
            }
        };
        if show.is_none() {
            println!("Did not get a show from the tvmaze api for tvmaze_show_id {}, so skipping", program.tvmaze_show_id);
            debug!("Did not get a show from the tvmaze api for tvmaze_show_id {}, so skipping", program.tvmaze_show_id);
            continue;
        }

        let program_to_update = to_program(show.unwrap());

        if program != program_to_update {
            something_changed = true;
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

        let mut episodes_to_insert: Vec<program_guide::Episode> = Vec::new();
        let episodes = match tvmaze_api.get_episodes(program.tvmaze_show_id) {
            Ok(e) => e,
            Err(error) => {
                eprintln!("Error getting episodes for tvmaze_show_id {} from the tvmaze api: {}", program.tvmaze_show_id, error);
                exit(exitcode::SOFTWARE);
            }
        };
        if episodes.is_none() {
            println!("Did not get any episodes for tvmaze_show_id {} from the tvmaze api, so skipping", program.tvmaze_show_id);
            continue;
        }

        let episodes = episodes.unwrap();

        for episode in episodes {
            let current_episode = match database.get_episode_by_episode_number(program.tvmaze_show_id, episode.season, episode.number) {
                Ok(e) => e,
                Err(error) => {
                    eprintln!("Error getting episode for id {}, season {}, number {} from the database: {}", program.tvmaze_show_id, episode.season, episode.number, error);
                    None
                }
            };

            let new_episode = to_episode(program.tvmaze_show_id, episode);

            if current_episode.is_some() {
                let current_episode = current_episode.unwrap();
                if current_episode != new_episode {
                    something_changed = true;
                    println!("episode before: {:?}", current_episode);
                    println!("episode after:  {:?}", new_episode);
                    debug!("episode before: {:?}", current_episode);
                    debug!("episode after:  {:?}", new_episode);
                }
            } else {
                something_changed = true;
                println!("New: {:?}", new_episode);
                debug!("New: {:?}", new_episode);
            }

            episodes_to_insert.push(new_episode);
        }

        let delete_result = match database.delete_episodes_by_program_id(program.tvmaze_show_id) {
            Ok(count) => (count, true),
            Err(error) => {
                eprintln!("Error deleting episodes for id {}: {}", program.tvmaze_show_id, error);
                (0, false)
            }
        };
        debug!("Delete episodes affected rows: {}", delete_result.0);

        if delete_result.1 {
            let insert_count = match database.insert_episodes_by_program_id(episodes_to_insert) {
                Ok(count) => count,
                Err(error) => {
                    eprintln!("Error inserting episodes for id {}: {}", program.tvmaze_show_id, error);
                    0
                }
            };
            debug!("Insert episodes affected rows: {}", insert_count);
            if delete_result.0 != insert_count {
                something_changed = true;
                println!("Episode count went from {} to {}", delete_result.0, insert_count);
            }
        } else {
            debug!("Delete episodes error'd, so skipping episodes insert");
        }

        if !something_changed {
            println!("No changes")
        }
    }

    exit(exitcode::OK);
}

// Do some conversion work.
fn to_program(show: Show) -> Program {
    let network = match show.network {
        Some(network) => network.name,
        None => {
            match show.web_channel {
                Some(web) => web.name,
                None => "".to_string()
            }
        }
    };

    // Some values are empty strings and we prefer nulls or Nones.
    let name = match show.name.trim().is_empty() {
        true => None,
        false => Some(show.name)
    };

    let url = match show.url.trim().is_empty() {
        true => None,
        false => Some(show.url)
    };

    Program {
        tvmaze_show_id: show.id,
        name,
        url,
        network: Some(network),
        last_update: Some(show.updated),
    }
}

// Do some conversion work.
fn to_episode(tvmaze_show_id: u32, episode: tvmaze::Episode) -> program_guide::Episode {
    let original_air_date = match episode.airdate.trim().is_empty() {
        true => None,
        false => Some(episode.airdate)
    };

    let title = match episode.name.trim().is_empty() {
        true => None,
        false => Some(episode.name)
    };

    let summary_url = match episode.url.trim().is_empty() {
        true => None,
        false => Some(episode.url)
    };

    program_guide::Episode {
        tvmaze_show_id,
        season: episode.season,
        number: episode.number,
        original_air_date,
        title,
        summary_url,
    }
}