use std::fs::File;
use std::io::Write;
use serde::{Deserialize, Serialize};
use rand::Rng;

#[derive(Serialize, Deserialize)]
pub struct BidData {
    pub num_inputs: usize,
    pub domain_size: usize,
    pub bids: Vec<u32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = std::env::args().nth(1).unwrap_or_else(|| "bid_data.bin".to_string());
    
    let num_inputs = 100;
    let domain_size: usize = 1024;  // Explicitly specify usize type
    
    println!("Generating {} random bids from domain [0, {}]...", num_inputs, domain_size - 1);
    
    // Generate random bids - Fixed for rand 0.7.3
    let mut rng = rand::thread_rng();
    let bids: Vec<u32> = (0..num_inputs)
        .map(|_| rng.gen_range(0, domain_size as u32))  // Cast to u32 for gen_range
        .collect();
    
    // Package bid data
    let bid_data = BidData {
        num_inputs,
        domain_size,  // Now both are usize
        bids,
    };
    
    // Save to file
    let encoded = bincode::serialize(&bid_data)?;
    let mut file = File::create(&filename)?;
    file.write_all(&encoded)?;
    
    println!("Bid generation complete!");
    println!("Generated bids: {:?}", &bid_data.bids[..10.min(bid_data.bids.len())]);
    if bid_data.bids.len() > 10 {
        println!("... and {} more", bid_data.bids.len() - 10);
    }
    println!("Data size: {:.2} KB", encoded.len() as f64 / 1024.0);
    println!("Saved to: {}", filename);
    
    Ok(())
}
