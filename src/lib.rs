//! This crate contains tools you can use to manage discord invite links.  
//!   
//! You can search google for every web page referring discord.gg in the last hour with google::search().  
//! After you got these links, you can load the pages and parse them to get discord invite links with intermediary::resolve().  
//! You can parse a discord invitation page with the Invite struct.
//!
//! # Examples
//!
//! ```no_run
//! use discord_finder::*;
//!
//! for page in 0..4 {
//!     for link in google::search(page).unwrap() {
//!         println!("resolving {}", link);
//!         for invite_link in intermediary::resolve(&link).unwrap() {
//!             println!("invite link found: {}", invite_link);
//!         }
//!     }
//! }
//! ```

#[derive(Debug)]
pub enum Error {
    Timeout,
    InvalidResponse,
}

/// Contains functions related to google pages parsing.
pub mod google {
    use super::Error;
    use string_tools::{get_all_after, get_all_between_strict};

    fn get_full_url(page: usize) -> String {
        format!(
            "https://www.google.com/search?q=\"discord.gg\"&tbs=qdr:h&filter=0&start={}",
            page * 10
        )
    }

    /// Search google for a something and returns result urls.  
    /// See [Google Advanced Search](https://www.google.com/advanced_search) for more information about request syntax.  
    /// Only one page is loaded.  
    ///   
    /// # Examples
    ///   
    /// ```
    /// use discord_finder::google;
    ///
    /// // note that we only test the first page of google results and that there can be more
    /// let links = google::search(0).unwrap();
    /// # assert!(!links.is_empty());
    /// ```
    pub fn search(page: usize) -> Result<Vec<String>, Error> {
        if let Ok(response) = minreq::get(get_full_url(page))
            .with_header("Accept", "text/plain")
            .with_header("Host", "www.google.com")
            .with_header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:71.0) Gecko/20100101 Firefox/71.0",
            )
            .send()
        {
            if let Ok(mut body) = response.as_str() {
                let mut rep = Vec::new();
                while let Some(url) =
                    get_all_between_strict(body, "\"r\"><a href=\"", "\" onmousedown=\"return rwt(")
                {
                    rep.push(url.to_string());
                    body = get_all_after(body, url);
                }
                Ok(rep)
            } else {
                Err(Error::InvalidResponse)
            }
        } else {
            Err(Error::Timeout)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn get_full_url_test() {
            assert_eq!(
                "https://www.google.com/search?q=\"discord.gg\"&tbs=qdr:h&filter=0&start=10",
                get_full_url(1)
            );
        }
    }
}

pub mod intermediary {
    use super::Error;
    use super::discord::get_invite_code;
    use string_tools::get_all_after;

    /// put an url+noise, get url (without http://domain.something/)
    fn get_url(url: &str) -> &str {
        let mut i = 0;
        for c in url.chars() {
            // todo %20
            if !c.is_ascii_alphanumeric() && c != '-' && c != '/' && c != '_' {
                break;
            }
            i += 1;
        }
        &url[..i]
    }

    pub fn resolve(url: &str) -> Result<Vec<String>, Error> {
        if let Ok(response) = minreq::get(url)
            .with_header("Accept", "text/plain")
            .with_header(
                "User-Agent",
                "Mozilla/5.0 (X11; Linux x86_64; rv:71.0) Gecko/20100101 Firefox/71.0",
            )
            .send()
        {
            if let Ok(mut body) = response.as_str() {
                let mut rep = Vec::new();
                // TODO discord.com
                while get_all_after(&body, "discord.gg/") != "" {
                    let url = get_url(get_all_after(&body, "discord.gg/"));
                    body = get_all_after(&body, "discord.gg/");
                    let url = if url.len() == 7 {
                        format!("https://discord.com/invite/{}", url)
                    } else {
                        continue;
                    };
                    if !rep.contains(&url) {
                        rep.push(url);
                    }
                }
                Ok(rep)
            } else {
                Err(Error::InvalidResponse)
            }
        } else {
            Err(Error::Timeout)
        }
    }
}

/// Contains discord fetcher
pub mod discord {
    use super::Error;
    use serde_json::{from_str, Value};
    use std::thread::sleep;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use string_tools::{get_all_between_strict, get_idx_between_strict};

    use serde::{Deserialize, Serialize};

    /// Extract the id of the invitation from an url.
    pub fn get_invite_code(url: &str) -> Option<&str> {
        if url.len() > 27 && &url[0..27] == "https://discord.com/invite/" {
            return Some(&url[27..]);
        } else if url.len() > 19 && &url[0..19] == "https://discord.gg/" {
            return Some(&url[19..]);
        }
        None
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Guild {
        #[serde(skip_serializing_if = "Option::is_none")]
        banner: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        splash: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        vanity_url_code: Option<String>,
        verification_level: u8,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Channel {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        r#type: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct User {
        id: String,
        username: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        avatar: Option<String>,
        discriminator: String,
    }

    /// A simple struct used to store informations about a discord server invite link.
    /// Can be serialized by activing the feature "serde-support"
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Invite {
        pub code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub guild: Option<Guild>,
        pub channel: Channel,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub inviter: Option<User>,
        pub approximate_member_count: u64,
        pub approximate_presence_count: u64,
    }

    impl Invite {
        /// Loads a discord.gg page and produces an Invite struct.
        pub fn fetch(url: &str) -> Result<Invite, Error> {
            let invite_code = match get_invite_code(url) {
                Some(code) => code,
                None => return Err(Error::InvalidResponse),
            };
            let url = format!("https://discord.com/api/v6/invites/{}?with_counts=true", invite_code);

            if let Ok(response) = minreq::get(&url)
                .with_header("Host", "discord.com")
                .with_header(
                    "User-Agent",
                    "Mozilla/5.0 (X11; Linux x86_64; rv:72.0) Gecko/20100101 Firefox/72.0",
                )
                .with_header("Accept", "text/html")
                .with_header("DNT", "1")
                .with_header("Connection", "keep-alive")
                .with_header("Upgrade-Insecure-Requests", "1")
                .with_header("TE", "Trailers")
                .send()
            {
                if response.status_code == 200 {
                    if let Ok(body) = response.as_str() {
                        println!("{}", body);
    
                        match from_str(body) {
                            Ok(invite) => Ok(invite),
                            Err(e) => {
                                eprintln!("Parsing error: {:?}", e);
                                Err(Error::InvalidResponse)
                            }
                        }
                    } else {
                        Err(Error::InvalidResponse)
                    }
                } else {
                    Err(Error::InvalidResponse)
                }
            } else {
                Err(Error::Timeout)
            }
        }

        /// Return the url
        pub fn get_url(&self) -> String {
            format!("https://discord.com/invite/{}", self.code)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_invite_struct() {
            let invite =
                Invite::fetch("https://discord.com/invite/seaofthievescommunity")
                    .unwrap();
            println!("{:#?}", invite);

            sleep(Duration::from_secs(5));

            let invite = Invite::fetch("https://discord.com/invite/UNWEj54").unwrap();
            println!("{:#?}", invite);

            sleep(Duration::from_secs(5));

            let invite =
            Invite::fetch("https://discord.gg/Yyakf3").unwrap();
            println!("{:#?}", invite);
        }

        #[test]
        fn get_invite_urls() {
            assert_eq!(
                get_invite_code("https://discord.com/invite/seaofthievescommunity"),
                Some("seaofthievescommunity")
            );
            assert_eq!(
                get_invite_code("https://discord.com/invite/UNWEj54"),
                Some("UNWEj54")
            );
            assert_eq!(
                get_invite_code("https://discord.gg/8j8b2xR"),
                Some("8j8b2xR")
            );
            assert_eq!(
                get_invite_code("https://discord.gg/Yyakf3"),
                Some("Yyakf3")
            );
        }
    }
}
