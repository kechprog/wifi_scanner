use serde::{Deserialize, Serialize};
use std::{cmp, process::Command};

#[derive(Serialize, Deserialize, Debug)]
struct SpeedTestResult {
    pub download: f64,
    pub upload: f64,
    pub ping: f64,
}

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn get_wifi_nets() -> Res<Vec<String>> {
    let output = Command::new("nmcli")
        .arg("device")
        .arg("wifi")
        .arg("list")
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    let ssid_start_index = output_str
        .lines()
        .next()
        .and_then(|line| line.find(" SSID"))
        .expect("Problem finding SSID")
        + 1; // Adding 5 to move past the "SSID" text

    let ssids = output_str
        .lines()
        .skip(1) // Skip the header line
        .filter_map(|line| {
            if line.len() > ssid_start_index {
                let ssid_end_index = line[ssid_start_index..]
                    .find(' ')
                    .map(|idx| idx + ssid_start_index)
                    .unwrap_or_else(|| line.len());

                let ssid = &line[ssid_start_index..ssid_end_index];
                if ssid != "--" {
                    Some(ssid.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Ok(ssids)
}

fn known_nets() -> Vec<String> {
    let output = Command::new("nmcli")
        .arg("connection")
        .arg("show")
        .output()
        .expect("Problem finding known nets");

    let output = String::from_utf8_lossy(&output.stdout);

    let known = output.lines().skip(1).map(|l| {
        let idx = l.find(" ").expect("Something is very wrong in known nets");
        let name = &l[0..idx];

        name.to_string()
    });

    known.collect()
}

fn intersection<T>(l1: &[T], l2: &[T]) -> Vec<T>
where
    T: cmp::Eq + Clone,
{
    let mut inter = vec![];
    for e1 in l1.iter() {
        for e2 in l2.iter() {
            if e1 == e2 {
                inter.push(e1.clone());
            }
        }
    }

    inter
}

fn try_connect(ssid: &str) -> Option<()> {
    let output = Command::new("nmcli")
        .arg("device")
        .arg("wifi")
        .arg("connect")
        .arg(ssid)
        .output()
        .ok()?;

    let _output = String::from_utf8_lossy(&output.stdout);

    Some(())
}

fn test_speed() -> Option<SpeedTestResult> {
    let output = Command::new("speedtest-cli")
        .arg("--no-upload")
        .arg("--json")
        .arg("--timeout")
        .arg("3")
        .arg("--secure")
        .output()
        .ok()?;

    let output = String::from_utf8_lossy(&output.stdout);

    serde_json::from_str(&output).ok()
}

fn main() -> Res<()> {
    let avaible_nets = get_wifi_nets()?;
    let known_nets = known_nets();
    let connectable = intersection(&avaible_nets, &known_nets);

    println!("Will check those:");
    for net in connectable.iter() {
        println!("\t{}", net)
    }

    let mut best = (&"".to_string(), f64::MIN);
    let all_work = connectable.len();
    let mut done = 0usize;

    for ssid in connectable.iter() {
        let res = if let Some(_) = try_connect(&ssid) {
            let download = if let Some(info) = test_speed() {
                info.download
            } else {
                f64::MIN
            };
            (ssid, download)
        } else {
            (ssid, f64::MIN)
        };

        if res.1 > best.1 {
            best = res;
        }

        done += 1;
        println!("[{}]", "#".repeat(done) + &"-".repeat(all_work - done))
    }

    println!("Fastest net is {} and has speed of {}", best.0, best.1);

    Ok(())
}
