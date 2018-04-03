extern crate clap;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::net::UdpSocket;
use std::fmt::Write;
use std::str;

use clap::{Arg, App};

mod ruggers;
use ruggers::{RuggedRecord, RuggedGeneration};

#[derive(Serialize, Deserialize)]
enum Command {
    Get   ( String ),
    Set   ( String, String ),
    Merge ( String, String, u64 ),
    SnapCreate ( String ),
    SnapGet    ( String, String ),
    SnapDelete ( String ),
    Ok,
    Error ( String ),
    Value ( String, String ),
}


fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Michael Ziegler <diese-addy@funzt-halt.net>")
        .about("Ruggers in-memory cache")
        .arg(Arg::with_name("node-id")
            .short("n")
            .long("node-id")
            .help("My Node ID.")
            .default_value("1")
        )
        .arg(Arg::with_name("listen")
            .short("l")
            .long("listen")
            .help("Listen address.")
            .default_value("[::]:22422")
        )
        .arg(Arg::with_name("replication_targets")
            .short("r")
            .long("reptargets")
            .takes_value(true)
            .multiple(true)
            .help("Replication targets to send writes to [none].")
        )
        .get_matches();

    let node_id: u8 = matches.value_of("node-id").unwrap().parse()
            .expect("node-id is not an int");

    if node_id > 15 {
        println!("Node ID must not exceed 15");
        return;
    }

    let mut datastore = RuggedGeneration::new_root(node_id);
    let mut snapshots = HashMap::new();

    let socket = UdpSocket::bind(matches.value_of("listen").unwrap()).unwrap();

    let mut buf = [0; 1024*1024];

    loop {
        let (len, src) = socket.recv_from(&mut buf)
            .expect("Couldn't read from socket");

        let resp = match serde_json::from_str::<Command>(std::str::from_utf8(&buf[..len])
            .expect("send me utf8 please"))
        {
            Ok(cmd) => {
                match cmd {
                    Command::Get(key) => {
                        if let Some(val) = datastore.get(&key) {
                            Some(Command::Value(key, val.as_ref().value.to_owned()))
                        } else {
                            Some(Command::Value(key, String::from("")))
                        }
                    }
                    Command::Set(key, val) => {
                        datastore = datastore.store(&key, &val);
                        if let Some(targets) = matches.values_of("replication_targets") {
                            let repl_cmd = Command::Merge(key, val, datastore.this_gen());
                            let repl_data = serde_json::to_string(&repl_cmd)
                                .expect("Couldn't encode replication command");
                            for argument in targets {
                                println!("Replicating to {}", argument);
                                socket.send_to(&repl_data[..].as_bytes(), &argument)
                                    .expect("Couldn't send");
                            }
                        }
                        Some(Command::Ok)
                    }
                    Command::Merge(key, val, gen) => {
                        match datastore.merge(RuggedRecord::new(gen, key, val)) {
                            Some(new_gen) => {
                                datastore = new_gen;
                                Some(Command::Ok)
                            }
                            None => {
                                Some(Command::Error(String::from("Merge conflict")))
                            }
                        }
                    }
                    Command::SnapCreate(snapname) => {
                        snapshots.insert(snapname, datastore.clone());
                        Some(Command::Ok)
                    }
                    Command::SnapGet(snapname, key) => {
                        if let Some(snap_datastore) = snapshots.get(&snapname) {
                            if let Some(val) = snap_datastore.get(&key) {
                                Some(Command::Value(key, val.as_ref().value.to_owned()))
                            } else {
                                Some(Command::Value(key, String::from("")))
                            }
                        } else {
                            Some(Command::Value(key, String::from("")))
                        }
                    }
                    Command::SnapDelete(snapname) => {
                        snapshots.remove(&snapname);
                        Some(Command::Ok)
                    }
                    Command::Ok => None,
                    Command::Value(_, _) => Some(Command::Error(String::from("Use Set to store something"))),
                    Command::Error(err) => {
                        println!("Received an error: {}", err);
                        None
                    }
                }
            }
            Err(err) => {
                Some(Command::Error(err.to_string()))
            }
        };

        if let Some(resp) = resp {
            let mut ser_data = serde_json::to_string(&resp)
                .expect("Couldn't encode response");
            ser_data.write_str("\n")
                .expect("Can't append newline to data :s");
            socket.send_to(&ser_data[..].as_bytes(), &src)
                .expect("Couldn't send");
        }
    }
}
