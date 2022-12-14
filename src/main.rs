use rincron::Rincron;

mod rincron;

fn main() {
    let mut rincron = Rincron::new().unwrap_or_else(|_| std::process::exit(1));
    rincron
        .read_config()
        .unwrap_or_else(|_| std::process::exit(2));
    rincron.execute();
}
