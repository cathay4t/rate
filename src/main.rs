// SPDX-License-Identifier: Apache-2.0

use bytesize::ByteSize;
use nispor::NetState;
use std::collections::HashMap;
use std::io::Read;

const INTERVAL: u64 = 1000;

fn main() {
    let matches = clap::Command::new("rate")
        .version("1.0")
        .author("Gris Ge <cnfourt@gmail.com>")
        .about("Show the realtime network speed")
        .arg(
            clap::Arg::new("NIC")
                .help("Show specific network interface only")
                .action(clap::ArgAction::Set)
                .index(1),
        )
        .arg(
            clap::Arg::new("repeat")
                .short('t')
                .action(clap::ArgAction::SetTrue)
                .help("Repeat"),
        )
        .get_matches();

    let mut filter = nispor::NetStateFilter::minimum();
    let iface_filter = nispor::NetStateIfaceFilter::minimum();
    filter.iface = Some(iface_filter);

    if let Some(iface_name) = matches.get_one::<String>("NIC") {
        if !std::path::Path::new(&format!("/sys/class/net/{iface_name}"))
            .exists()
        {
            eprintln!(
                "FAIL: Specific interface {} does not exists",
                iface_name
            );
            std::process::exit(1);
        }
        if matches.get_flag("repeat") {
            loop {
                show_result(iface_name, get_net_speed(iface_name));
            }
        } else {
            show_result(iface_name, get_net_speed(iface_name));
        }
    } else if matches.get_flag("repeat") {
        let net_state = NetState::retrieve_with_filter(&filter).unwrap();
        loop {
            show_all(&net_state);
        }
    } else {
        let net_state = NetState::retrieve_with_filter(&filter).unwrap();
        show_all(&net_state);
    }
}

fn get_net_speed(iface_name: &str) -> (u64, u64) {
    let (cur_rx, cur_tx) = get_net_bytes(iface_name);
    std::thread::sleep(std::time::Duration::from_millis(INTERVAL));
    let (new_rx, new_tx) = get_net_bytes(iface_name);
    (
        (new_rx - cur_rx) * 1000 / INTERVAL,
        (new_tx - cur_tx) * 1000 / INTERVAL,
    )
}

fn get_net_bytes(iface_name: &str) -> (u64, u64) {
    let rx_file = format!("/sys/class/net/{}/statistics/rx_bytes", iface_name);
    let tx_file = format!("/sys/class/net/{}/statistics/tx_bytes", iface_name);
    if std::path::Path::new(&rx_file).exists() {
        (read_sysfs_as_u64(&rx_file), read_sysfs_as_u64(&tx_file))
    } else {
        (0, 0)
    }
}

fn read_sysfs_as_u64(file_path: &str) -> u64 {
    let content = read_file(file_path);
    content.trim().parse::<u64>().unwrap()
}

fn show_result(iface_name: &str, (rx_speed, tx_speed): (u64, u64)) {
    let rx_speed_str = ByteSize::b(rx_speed).to_string_as(true);
    let tx_speed_str = ByteSize::b(tx_speed).to_string_as(true);

    println!(
        "{:>8}: v {:>9}/s ^ {:>9}/s",
        iface_name, rx_speed_str, tx_speed_str
    );
}

fn read_file(file_path: &str) -> String {
    let mut fd =
        std::fs::File::open(file_path).expect("Failed to open config file");
    let mut contents = String::new();
    fd.read_to_string(&mut contents)
        .expect("Failed to read config file");
    contents
}

fn show_all(net_state: &NetState) {
    let cur_all_bytes = get_all_bytes(net_state);

    std::thread::sleep(std::time::Duration::from_millis(1000));

    let new_all_bytes = get_all_bytes(net_state);

    for (iface_name, (cur_rx, cur_tx)) in cur_all_bytes.iter() {
        let (new_rx, new_tx) = new_all_bytes[iface_name];
        let (rx_speed, tx_speed) = (new_rx - cur_rx, new_tx - cur_tx);
        if rx_speed == 0 && tx_speed == 0 {
            continue;
        }
        show_result(iface_name, (rx_speed, tx_speed));
    }
}

fn get_all_bytes(net_state: &NetState) -> HashMap<&str, (u64, u64)> {
    let mut all_bytes = HashMap::new();
    for iface_name in net_state.ifaces.keys() {
        if should_skip(iface_name) {
            continue;
        }
        all_bytes.insert(iface_name.as_str(), get_net_bytes(iface_name));
    }
    all_bytes
}

fn should_skip(iface_name: &str) -> bool {
    iface_name == "lo"
        || iface_name.starts_with("vnet")
        || iface_name.starts_with("virbr")
}
