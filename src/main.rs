use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use clap::Parser;
use std::{
    env, io,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use ureq::Response;

extern crate confy;

#[macro_use]
extern crate serde_derive;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct RefreshTokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct ResponseError {
    error: String,
    error_description: String,
}

#[derive(Serialize, Deserialize)]
struct SpotifyAlbumImage {
    url: String,
    height: u32,
    width: u32,
}

#[derive(Serialize, Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct SpotifyAlbum {
    name: String,
    images: Vec<SpotifyAlbumImage>,
}

#[derive(Serialize, Deserialize)]
struct SpotifyItem {
    artists: Vec<SpotifyArtist>,
    album: SpotifyAlbum,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct PlaybackState {
    is_playing: bool,
    item: SpotifyItem,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: String::from(""),
            client_secret: String::from(""),
            redirect_uri: String::from(""),
            access_token: String::from(""),
            refresh_token: String::from(""),
            expires_at: 0,
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Use this flag to run the initial Spotify login process
    #[arg(short, long)]
    setup: bool,

    /// Use this flag to output the information to a file in JSON format
    #[arg(short, long)]
    json: bool,
}

#[derive(PartialEq)]
enum Action {
    DEFAULT,
    SETUP,
    PLAYBACK,
}

fn config_load(custom_path: Option<String>) -> Result<Config, String> {
    if let Some(_) = custom_path {
        Err(String::from("no impl"))
    } else {
        let cfg: Config = confy::load("tail_spotify", None).map_err(|e| e.to_string())?;
        let file =
            confy::get_configuration_file_path("tail_spotify", None).map_err(|e| e.to_string())?;
        Ok(cfg)
    }
}

fn config_save(custom_path: Option<String>, config: Config) -> Result<(), String> {
    if let Some(_) = custom_path {
        Err(String::from("no impl"))
    } else {
        confy::store("tail_spotify", None, config).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn get_tokens(auth_code: String, config: &mut Config) -> Result<TokenResponse, String> {
    let request = ureq::post("https://accounts.spotify.com/api/token")
        .set(
            "Authorization",
            &format!(
                "Basic {}",
                URL_SAFE.encode(format!("{}:{}", config.client_id, config.client_secret))
            ),
        )
        .send_form(&[
            ("grant_type", "authorization_code"),
            ("code", &auth_code),
            ("redirect_uri", &config.redirect_uri),
        ]);
    let failed_response: Option<Response>;
    match request {
        Ok(response) => {
            return Ok(response.into_json().map_err(|e| e.to_string())?);
        }
        Err(response_err) => failed_response = response_err.into_response(),
    }
    if let Some(response) = failed_response {
        let error_data: ResponseError = response.into_json().map_err(|e| e.to_string())?;
        return Err(format!("Spotify returned the following while authenticating your account: {} ({}), please try again.", error_data.error_description, error_data.error));
    }
    return Err(String::from("Unknown Error"));
}

fn refresh_tokens(config: &mut Config) -> Result<(), String> {
    if config.access_token != "" && config.refresh_token != "" && config.expires_at != 0 {
        let request = ureq::post("https://accounts.spotify.com/api/token")
            .set(
                "Authorization",
                &format!(
                    "Basic {}",
                    URL_SAFE.encode(format!("{}:{}", config.client_id, config.client_secret))
                ),
            )
            .send_form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &config.refresh_token),
            ]);
        let failed_response: Option<Response>;
        match request {
            Ok(response) => {
                let refresh_data: RefreshTokenResponse =
                    response.into_json().map_err(|e| e.to_string())?;
                config.access_token = refresh_data.access_token;
                let expiry_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
                    + Duration::from_secs(refresh_data.expires_in);
                config.expires_at = expiry_time.as_secs();
                return Ok(());
            }
            Err(response_err) => failed_response = response_err.into_response(),
        }
        if let Some(response) = failed_response {
            let error_data: ResponseError = response.into_json().map_err(|e| e.to_string())?;
            return Err(format!("Spotify returned the following while refreshing access to your account: {} ({}), please try again.", error_data.error_description, error_data.error));
        }
        return Err(String::from("Unknown Error"));
    } else {
        return Err(String::from("Unknown Error"));
    }
}

fn get_playback(config: &mut Config) -> Result<Option<PlaybackState>, String> {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let expiry_time = Duration::from_secs(config.expires_at);
    if current_time < expiry_time {
        let request = ureq::get("https://api.spotify.com/v1/me/player")
            .set("Authorization", &format!("Bearer {}", config.access_token))
            .call();
        let failed_response: Option<Response>;
        match request {
            Ok(response) => match response.status() {
                200 => return Ok(Some(response.into_json().map_err(|e| e.to_string())?)),
                _ => return Ok(None),
            },
            Err(response_err) => failed_response = response_err.into_response(),
        }
        if let Some(response) = failed_response {
            let error_data: ResponseError = response.into_json().map_err(|e| e.to_string())?;
            return Err(format!("Spotify returned the following while requesting the playback state on behalf of your account: {} ({}), please try again.", error_data.error_description, error_data.error));
        }
    } else {
        refresh_tokens(config)?;
        return get_playback(config);
    }
    return Err(String::from("Unknown Error"));
}

fn main() {
    let args = Args::parse();
    let mut config_path: Option<String> = None;
    let mut mode: Action = Action::DEFAULT;
    if args.setup {
        mode = Action::SETUP
    } else if args.json {
        mode = Action::PLAYBACK;
    }
    let config = config_load(config_path);
    if let Ok(mut cfg) = config {
        if mode == Action::SETUP || (cfg.access_token == "" || cfg.refresh_token == "") {
            println!("Authroize your Spotify account via: https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&scope=user-read-currently-playing%20user-read-playback-state", cfg.client_id, cfg.redirect_uri);
            let mut auth_code = String::new();
            match io::stdin().read_line(&mut auth_code) {
                Ok(_) => auth_code = auth_code.trim().to_string(),
                Err(_) => {}
            }
            if auth_code == "" {
                panic!("There was an error parsing your input");
            }
            match get_tokens(auth_code, &mut cfg) {
                Ok(token_data) => {
                    cfg.access_token = token_data.access_token.clone();
                    cfg.refresh_token = token_data.refresh_token.clone();
                    let expiry_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
                        + Duration::from_secs(token_data.expires_in);
                    cfg.expires_at = expiry_time.as_secs();
                }
                Err(auth_err) => {
                    panic!("Error: {}", auth_err);
                }
            }
            match config_save(None, cfg) {
                Ok(_) => {
                    println!("Config Saved.")
                }
                Err(cfg_err) => {
                    panic!(
                        "There was an error while saving the config file: {}",
                        cfg_err
                    );
                }
            }
        } else if mode == Action::DEFAULT || mode == Action::PLAYBACK {
            let playback = get_playback(&mut cfg);
            let cfg_status = config_save(None, cfg);
            match cfg_status {
                Ok(_) => {}
                Err(cfg_err) => {
                    panic!(
                        "There was an error while saving the config file: {}",
                        cfg_err
                    );
                }
            }
            match playback {
                Ok(playback_status) => {
                    match playback_status {
                        Some(playback_data) => {
                            if mode == Action::DEFAULT && playback_data.is_playing {
                                println!(
                                    "{} - {}",
                                    playback_data.item.name, playback_data.item.artists[0].name
                                );
                            } else if mode == Action::PLAYBACK {
                                let stringified: Result<String, String> =
                                    serde_json::to_string(&playback_data)
                                        .map_err(|e| e.to_string());
                                match stringified {
                                    Ok(json_data) => println!("{}", json_data),
                                    Err(err_data) => println!("There was an error converting the playback data to JSON: {}", err_data),
                                }
                            } else {
                                println!("No Music Playing");
                            }
                        }
                        None => println!("No Music Playing"),
                    }
                }
                Err(playback_err) => println!("{}", playback_err),
            }
        }
    } else {
        println!(
            "There was an error loading the config: {}",
            config.unwrap_err()
        )
    }
}
