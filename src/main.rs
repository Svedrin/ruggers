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
struct CmdPacket {
    cmd: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    val: String,
    #[serde(default)]
    gen: u64
}

#[derive(Serialize, Deserialize)]
struct CmdResult {
    res: String,
    key: String,
    val: String,
    gen: u64
}

fn main() {
    let mut gen = RuggedGeneration::new_root(1);

    let socket = UdpSocket::bind("[::]:22422").unwrap();

    let mut buf = [0; 1024*1024];

    loop {
        let (len, src) = socket.recv_from(&mut buf)
            .expect("Couldn't read from socket");

        let resp = match serde_json::from_str::<CmdPacket>(std::str::from_utf8(&buf[..len])
            .expect("send me utf8 please"))
        {
            Ok(cmd) => {
                if cmd.cmd == String::from("get") {
                    if let Some(val) = gen.get(&cmd.key) {
                        CmdResult {
                            res: String::from("get"),
                            key: cmd.key,
                            val: val.value.to_owned(),
                            gen: 0,
                        }
                    } else {
                        CmdResult {
                            res: String::from("get"),
                            key: cmd.key,
                            val: String::from(""),
                            gen: 0,
                        }
                    }
                }
                else if cmd.cmd == String::from("mrg") {
                    match gen.merge(RuggedRecord::new(cmd.gen, 0, cmd.key, cmd.val)) {
                        Some(new_gen) => {
                            gen = new_gen;
                            CmdResult {
                                res: String::from("ack"),
                                key: String::from(""),
                                val: String::from(""),
                                gen: gen.this_gen(),
                            }
                        }
                        None => {
                            CmdResult {
                                res: String::from("nak"),
                                key: String::from(""),
                                val: String::from(""),
                                gen: 0,
                            }
                        }
                    }
                }
                else if cmd.cmd == String::from("set") {
                    gen = gen.store(&cmd.key, &cmd.val);
                    CmdResult {
                        res: String::from("set"),
                        key: cmd.key,
                        val: String::from(""),
                        gen: 0,
                    }
                }
                else {
                    CmdResult {
                        res: String::from("nak"),
                        key: String::from(""),
                        val: String::from(""),
                        gen: 0,
                    }
                }
            }
            Err(err) => {
                CmdResult {
                    res: String::from("err"),
                    key: String::from(""),
                    val: err.to_string(),
                    gen: 0,
                }
            }
        };

        let mut ser_data = serde_json::to_string(&resp)
            .expect("Couldn't encode response");
        ser_data.write_str("\n");
        socket.send_to(&ser_data[..].as_bytes(), &src)
            .expect("Couldn't send");
    }
}
