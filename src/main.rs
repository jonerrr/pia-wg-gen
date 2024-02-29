use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::net::IpAddr;

mod config;
mod tests;

#[derive(Debug, Deserialize)]
struct ServerList {
    groups: HashMap<String, Vec<GroupDetails>>,
    regions: Vec<Region>,
}

#[derive(Debug, Deserialize)]
struct GroupDetails {
    ports: Vec<i32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Region {
    id: String,
    name: String,
    // dns: String,
    offline: bool,
    servers: HashMap<String, Vec<ServerDetails>>,
}

impl Region {
    // impliment sort by id
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerDetails {
    ip: IpAddr,
    cn: String,
}

#[derive(Debug, Deserialize)]
struct Token {
    token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        bail!(
            "\nUsage: {} <username> <password> <region ID (optional)>\nVersion: {}",
            args[0],
            env!("CARGO_PKG_VERSION")
        );
    }

    let mut login = HashMap::new();
    login.insert("username", &args[1]);
    login.insert("password", &args[2]);

    let token: Token = reqwest::Client::new()
        .post("https://www.privateinternetaccess.com/api/client/v2/token")
        .form(&login)
        .send()
        .await?
        .json()
        .await
        .expect("Failed to get token (are your username and password correct?)");

    let mut list: ServerList = {
        let list_raw = reqwest::Client::new()
            .get("https://serverlist.piaservers.net/vpninfo/servers/v6")
            .send()
            .await?
            .text()
            .await
            .expect("Failed to fetch server list");
        // remove base64 data at the end of the request so only the JSON is left
        serde_json::from_str(list_raw.split_once('\n').unwrap().0).unwrap()
    };

    // get the port to use to connect to Wireguard VPN
    let port = list
        .groups
        .get("wg")
        .unwrap()
        .first()
        .unwrap()
        .ports
        .first()
        .unwrap();

    // Check if user supplied a region ID
    let region = {
        if let Some(region) = args.get(3) {
            list.regions
                .iter()
                .find(|r| r.id == *region)
                .expect("Failed to find region by ID")
        } else {
            list.regions.sort_by(|r1, r2| r1.id.cmp(&r2.id));
            for (i, region) in list.regions.iter().enumerate() {
                println!("{}: {}", i, region.name);
            }

            println!("Select region by number:");
            let mut region_id = String::new();
            std::io::stdin().read_line(&mut region_id).unwrap();
            let region_id = region_id.trim().parse::<usize>().unwrap();
            &list.regions[region_id]
        }
    };

    if region.offline {
        bail!("Region is offline");
    }

    println!("Selected {} (ID: {})", region.name, region.id);

    let data = reqwest::Client::new()
        .get("https://raw.githubusercontent.com/pia-foss/manual-connections/master/ca.rsa.4096.crt")
        .send()
        .await?
        .bytes()
        .await
        .expect("Failed to fetch PIA certificate");
    println!("Fetched PIA certificate");

    let server = region.servers.get("wg").unwrap().first().unwrap();

    let pia_client = reqwest::Client::builder()
        .resolve(
            &server.cn,
            format!("{}:{}", server.ip, port).parse().unwrap(),
        )
        .add_root_certificate(reqwest::Certificate::from_pem(&data).unwrap())
        .build()?;

    let config = config::Config::new(
        &region.servers["wg"][0].cn,
        &token.token,
        *port,
        &pia_client,
    )
    .await?;

    config
        .write(format!("./wg-{}.conf", region.id).parse().unwrap())
        .await;

    Ok(())
}
