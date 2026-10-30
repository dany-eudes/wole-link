mod link;

fn main() {
    // Placeholder until network-card discovery lands: prints the link for a sample device.
    let sample = link::Device {
        mac: [0, 0, 0, 0, 0, 0],
        name: None,
        ip: None,
        broadcast: None,
        port: None,
    };
    println!("{}", link::build(&sample));
}
