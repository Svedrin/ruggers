extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;


use std::net::UdpSocket;
use std::fmt::Write;
use std::str;

mod ruggers;
use ruggers::{RuggedRecord, RuggedGeneration};

#[derive(Serialize, Deserialize)]
enum Command {
    Get   ( String ),
    Set   ( String, String ),
    Merge ( String, String, u64 ),
}

#[derive(Serialize, Deserialize)]
enum CmdResult {
    Ok,
    Error ( String ),
    Value ( String, String ),
}

fn main() {
    let mut datastore = RuggedGeneration::new_root(1);

    let socket = UdpSocket::bind("[::]:22422").unwrap();

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
                            CmdResult::Value(key, val.as_ref().value.to_owned())
                        } else {
                            CmdResult::Value(key, String::from(""))
                        }
                    }
                    Command::Set(key, val) => {
                        datastore = datastore.store(&key, &val);
                        CmdResult::Ok
                    }
                    Command::Merge(key, val, gen) => {
                        match datastore.merge(RuggedRecord::new(gen, 0, key, val)) {
                            Some(new_gen) => {
                                datastore = new_gen;
                                CmdResult::Ok
                            }
                            None => {
                                CmdResult::Error(String::from("Merge conflict"))
                            }
                        }
                    }
                }
            }
            Err(err) => {
                CmdResult::Error(err.to_string())
            }
        };

        let mut ser_data = serde_json::to_string(&resp)
            .expect("Couldn't encode response");
        ser_data.write_str("\n")
            .expect("Can't append newline to data :s");
        socket.send_to(&ser_data[..].as_bytes(), &src)
            .expect("Couldn't send");
    }
}
