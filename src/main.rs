use rspotify::{self, OAuth};

pub(crate) fn generate_random_string(length: usize, alphabet: &[u8]) -> String {
    let mut buf = vec![0u8; length];
    getrandom(&mut buf).unwrap();
    let range = alphabet.len();

    buf.iter()
        .map(|byte| alphabet[*byte as usize % range] as char)
        .collect()
}

fn create_auth(redirect_uri: String) -> Result<OAuth, String> {
    println!("{}", redirect_uri);
    let final_oauth = OAuth {
        redirect_uri: redirect_uri,
        state: generate_random_string(16, alphabets::ALPHANUM),
        scopes: HashSet::new(),
        proxies: None,
    };

    return Err("temp".to_string());
}

fn main() {
    println!("Hello, world!");
    let oauth = create_auth(String::from("temp.com"));
}
