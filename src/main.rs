use rincron::Rincron;

mod rincron;

fn main() {
    let mut rincron = Rincron::init().unwrap_or_else(|_| std::process::exit(1));
    let _ = rincron.read_config();
    rincron.execute();
}
