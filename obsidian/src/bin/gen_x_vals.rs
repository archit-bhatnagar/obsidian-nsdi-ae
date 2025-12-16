// generate_x_values.rs
use std::fs;
use rand::Rng;

fn main() {
    let num_clients = 100;
    let domain_size = 1280;
    let mut rng = rand::thread_rng();
    
    let x_values: Vec<u64> = (0..num_clients)
        .map(|_| rng.gen_range(0, domain_size))
        .collect();
    
    let content = x_values.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    
    fs::write("x_values.txt", content).expect("Failed to write x_values.txt");
    println!("Generated {} x values", num_clients);
}
