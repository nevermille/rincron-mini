use rincron::Rincron;

mod file_check;
mod rincron;
mod watch_element;
mod watch_manager;

fn main() {
    println!("Rincron-Mini Copyright (C) 2022-2023 Camille Nevermind");
    println!("THIS SOFTWARE IS DISTRIBUTED UNDER GPL-3.0 LICENSE");
    println!("THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND");
    println!("EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES");
    println!("OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.");

    let mut rincron = Rincron::init().unwrap_or_else(|_| std::process::exit(1));
    let _ = rincron.read_configs();
    rincron.execute();
}
