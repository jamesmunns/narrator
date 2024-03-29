use std::io::{self, Write};
use std::time::{Duration, Instant};
use kolben::rlercobs;

fn main() {
    let mut dgs_port = None;
    let ports = serialport::available_ports().unwrap();
    for port in ports {
        if let serialport::SerialPortType::UsbPort(usb_info) = port.port_type {
            if let Some(num) = usb_info.serial_number {
                if num.to_lowercase().contains("diegesis") {
                    dgs_port = Some(port.port_name);
                }
            }
        } else {
            continue;
        }
    }

    let dgs_port = dgs_port.unwrap();

    println!("Found diegesis on port: {}", dgs_port);

    let port = serialport::new(&dgs_port, 115_200)
        .timeout(Duration::from_millis(10))
        .open();

    let mut start = Instant::now();
    let mut bytes_rxd = 0;
    let mut bytes_dec = 0;
    let mut moving_avg_rxd = -1.0f64;
    let mut moving_avg_dec = -1.0f64;
    let mut last = 0u8;
    let mut current = Vec::new();

    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            println!("Receiving data on {}:", &dgs_port);
            loop {
                if start.elapsed() >= Duration::from_millis(250) {
                    if moving_avg_rxd <= 0.0 {
                        moving_avg_rxd = bytes_rxd as f64;
                    } else {
                        moving_avg_rxd *= 0.9;
                        moving_avg_rxd += (bytes_rxd as f64) * 0.1;
                    }

                    if moving_avg_dec <= 0.0 {
                        moving_avg_dec = bytes_dec as f64;
                    } else {
                        moving_avg_dec *= 0.9;
                        moving_avg_dec += (bytes_dec as f64) * 0.1;
                    }

                    println!(
                        "RX: {:0.02} KiB/sec {:0.02} KiB/sec (avg) DEC: {:0.02} KiB/sec {:0.02} KiB/sec (avg)",
                        4.0 * (bytes_rxd as f64) / 1024.0,
                        4.0 * moving_avg_rxd / 1024.0,
                        4.0 * (bytes_dec as f64) / 1024.0,
                        4.0 * moving_avg_dec / 1024.0,
                    );
                    bytes_rxd = 0;
                    bytes_dec = 0;
                    start = Instant::now();
                }

                match port.read(serial_buf.as_mut_slice()) {
                    Ok(t) => {
                        bytes_rxd += t;
                        current.extend_from_slice(&serial_buf[..t]);
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                    Err(e) => {
                        eprintln!("{:?}", e);
                        ::std::process::exit(1);
                    },
                }

                if let Some(pos) = current.iter().position(|b| *b == 0) {
                    let remainder = current.split_off(pos + 1);
                    let len = current.len();
                    let decoded = rlercobs::decode(&current[..len-1]).unwrap();
                    if let Err(e) = postcard::from_bytes::<diegesis_icd::DataReport>(&decoded) {
                        println!("{:?}", decoded);
                        println!("Decode error: {:?}", e);
                        println!("decoded: {}", decoded.len());
                        println!("current: {}", current.len());
                        println!("currentd: {:02X?}", current);
                    }
                    bytes_dec += decoded.len();
                    current = remainder;
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", &dgs_port, e);
            ::std::process::exit(1);
        }
    }
}
