// use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::net::IpAddr;

mod config;

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
    port_forward: bool,
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
async fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: {} <username> <password> <region ID>", args[0]);
        return Err(());
    }

    let mut login = HashMap::new();
    login.insert("username", &args[1]);
    login.insert("password", &args[2]);

    let token: Token = reqwest::Client::new()
        .post("https://www.privateinternetaccess.com/api/client/v2/token")
        .form(&login)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .expect("Failed to login");

    let mut list: ServerList = {
        let list_raw = reqwest::Client::new()
            .get("https://serverlist.piaservers.net/vpninfo/servers/v6")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        // remove base64 data at the end of the request so only the JSON is left
        serde_json::from_str(list_raw.split_once('\n').unwrap().0).unwrap()
    };

    // let region_ids = list.regions.iter().map(|r| &r.id).collect::<Vec<_>>();

    let port = list
        .groups
        .get("wg")
        .unwrap()
        .first()
        .unwrap()
        .ports
        .first()
        .unwrap();

    let region = {
        if args.len() != 4 {
            list.regions.sort_by(|r1, r2| r1.id.cmp(&r2.id));
            for (i, region) in list.regions.iter().enumerate() {
                println!("{}: {}", i, region.name);
            }

            println!("Select region by number:");
            // get stdin
            let mut region_id = String::new();
            std::io::stdin().read_line(&mut region_id).unwrap();
            let region_id = region_id.trim().parse::<usize>().unwrap();
            &list.regions[region_id]
        } else {
            list.regions
                .iter()
                .find(|r| r.id == args[3])
                .expect("Cant find region by id")
        }
    };

    println!("Selected {}", region.name);

    let data = reqwest::Client::new()
        .get("https://raw.githubusercontent.com/pia-foss/manual-connections/master/ca.rsa.4096.crt")
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .expect("Faile to fetch PIA certificate");
    println!("[INFO] Fetched PIA certificate");

    let server = region.servers.get("wg").unwrap().first().unwrap();

    let pia_client = reqwest::Client::builder()
        .resolve(
            &server.cn,
            format!("{}:{}", server.ip, port).parse().unwrap(),
        )
        .add_root_certificate(reqwest::Certificate::from_pem(&data).unwrap())
        .build()
        .expect("Failed to build http client");

    let config = config::Config::new(
        &region.servers["wg"][0].cn,
        &token.token,
        *port,
        &pia_client,
    )
    .await
    .unwrap();

    config
        .write(format!("./pia-wg-{}.conf", region.id).parse().unwrap())
        .await;

    Ok(())
}
