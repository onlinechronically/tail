use base64::{engine::general_purpose::URL_SAFE, Engine as _};
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
struct TokenResponseError {
    error: String,
    error_description: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbumImage {
    url: String,
    height: u32,
    width: u32,
}

#[derive(Debug, Deserialize)]
struct SpotifyArtist {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyAlbum {
    name: String,
    images: Vec<SpotifyAlbumImage>,
}

#[derive(Debug, Deserialize)]
struct SpotifyItem {
    artists: Vec<SpotifyArtist>,
    album: SpotifyAlbum,
    name: String,
}

#[derive(Debug, Deserialize)]
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

fn config_load(custom_path: Option<String>) -> Result<Config, String> {
    if let Some(_) = custom_path {
        Err(String::from("no impl"))
    } else {
        let cfg: Config = confy::load("tail_spotify", None).map_err(|e| e.to_string())?;
        let file =
            confy::get_configuration_file_path("tail_spotify", None).map_err(|e| e.to_string())?;
        println!("Config Loaded ({})", file.display());
        Ok(cfg)
    }
}

fn get_tokens(auth_code: String, config: &Config) -> Result<TokenResponse, String> {
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
            let token_data: TokenResponse = response.into_json().map_err(|e| e.to_string())?;
            return Ok(token_data);
        }
        Err(response_err) => failed_response = response_err.into_response(),
    }
    if let Some(response) = failed_response {
        let error_data: TokenResponseError = response.into_json().map_err(|e| e.to_string())?;
        return Err(format!("Spotify returned the following while authenticating your account: {} ({}), please try again.", error_data.error_description, error_data.error));
    }
    return Err(String::from("Unknown Error"));
}

fn get_playback(config: &Config) -> Result<(), String> {
    // get time
    // check if current time is less than expiry, fallback and refresh if not
    // use the
    Ok(())
}

fn main() {
    let mut input_args: Vec<String> = env::args().collect();
    dbg!(&input_args);
    input_args.remove(0);
    dbg!(&input_args);
    let mut config_path: Option<String> = None;
    for i in 0..input_args.len() {
        if (&input_args[i]).starts_with("") {
            if &input_args[i] == "--config" && config_path == None {
                config_path = Some(input_args[i + 1].clone());
            }
        }
    }
    let config = config_load(config_path);
    if let Ok(mut cfg) = config {
        if cfg.access_token == "" || cfg.refresh_token == "" {
            println!("Authroize your Spotify account via: https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&scope=user-read-currently-playing%20user-read-playback-state", cfg.client_id, cfg.redirect_uri);
            let mut auth_code = String::new();
            match io::stdin().read_line(&mut auth_code) {
                Ok(_) => auth_code = auth_code.trim().to_string(),
                Err(_) => {}
            }
            if auth_code == "" {
                panic!("There was an error parsing your input");
            }
            match get_tokens(auth_code, &cfg) {
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
        } else {
            println!("no impl (cycle)")
        }
    } else {
        println!("{}", config.unwrap_err())
    }
}
