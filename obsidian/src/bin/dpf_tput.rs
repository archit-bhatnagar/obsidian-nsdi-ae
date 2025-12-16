use std::time::Instant;
use std::fs::File;
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::sync::mpsc;
use std::thread;
use std::env;
use serde::{Deserialize, Serialize};
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
use rand::Rng;
use counttree::prg::FromRng;

#[derive(Serialize, Deserialize)]
pub struct AuctionData {
    pub num_clients: usize,
    pub domain_size: usize,
    pub updated_domain: usize,
    pub max_possible_sum: usize,
    
    // First FSS data
    pub values1_0: Vec<FE>,
    pub values1_1: Vec<FE>,
    pub values2_0: Vec<FE>,
    pub values2_1: Vec<FE>,
    pub r: FE,
    pub r_0: FE,
    pub r_1: FE,
    pub alpha_val_0: FE,
    pub alpha_val_1: FE,
    
    // Second FSS data
    pub col_sum_values1_0: Vec<FE>,
    pub col_sum_values1_1: Vec<FE>,
    pub col_sum_values2_0: Vec<FE>,
    pub col_sum_values2_1: Vec<FE>,
    pub r2: FE,
    pub r2_0: FE,
    pub r2_1: FE,
    pub alpha_val2_0: FE,
    pub alpha_val2_1: FE,
    pub alpha_r2_0: FE,
    pub alpha_r2_1: FE,
    
    // Third FSS data
    pub tie_values1_0: Vec<FE>,
    pub tie_values1_1: Vec<FE>,
    pub tie_values2_0: Vec<FE>,
    pub tie_values2_1: Vec<FE>,
    pub r3: FE,
    pub r3_0: FE,
    pub r3_1: FE,
    pub alpha_val3_0: FE,
    pub alpha_val3_1: FE,
    pub alpha_r3_0: FE,
    pub alpha_r3_1: FE,
    
    // Global values
    pub alpha_val: FE,
    pub x_val: Vec<u64>,
}

#[derive(Debug)]
struct BenchmarkResult {
    target_aps: u32,
    actual_aps: f64,
    p50_ms: f64,
    p99_ms: f64,
    samples: usize,
    mean_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

fn run_auction_protocol(data: &AuctionData) -> f64 {
    // Start timing the online phase
    let client_start = Instant::now();
    
    let mut all_client_s0 = vec![FE::zero(); data.domain_size];
    let mut all_client_s1 = vec![FE::zero(); data.domain_size];
    let mut all_client_m0 = vec![FE::zero(); data.domain_size];
    let mut all_client_m1 = vec![FE::zero(); data.domain_size];

    for client in 0..data.num_clients {
        let mut shifted_val_1_0 = vec![FE::zero(); data.domain_size];
        let mut shifted_val_1_1 = vec![FE::zero(); data.domain_size];
        let mut shifted_val_2_0 = vec![FE::zero(); data.domain_size];
        let mut shifted_val_2_1 = vec![FE::zero(); data.domain_size];
        for i in 0..data.domain_size {
            let idx = (i + data.x_val[client] as usize) % data.domain_size;
            shifted_val_1_0[i] = data.values1_0[idx].clone();
            shifted_val_1_1[i] = data.values1_1[idx].clone();
            shifted_val_2_0[i] = data.values2_0[idx].clone();
            shifted_val_2_1[i] = data.values2_1[idx].clone();
        }

        let mut cumulative_s0 = FE::zero();
        let mut cumulative_s1 = FE::zero();
        let mut cumulative_m0 = FE::zero();
        let mut cumulative_m1 = FE::zero();
        for i in 0..data.domain_size {
            cumulative_s0.add(&shifted_val_1_0[i]);
            all_client_s0[i].add(&cumulative_s0.clone());

            cumulative_s1.add(&shifted_val_1_1[i]);
            all_client_s1[i].add(&cumulative_s1.clone());

            cumulative_m0.add(&shifted_val_2_0[i]);
            all_client_m0[i].add(&cumulative_m0.clone());

            cumulative_m1.add(&shifted_val_2_1[i]);
            all_client_m1[i].add(&cumulative_m1.clone());
        }
    }

    let mut current_threshold = data.num_clients - 1;
    let mut second_highest_found = false;
    let mut second_highest_bid = 0;

    while current_threshold > 0 && !second_highest_found {
        let mut all_col_shifted_values = Vec::with_capacity(data.domain_size);
        
        for idx in 0..data.domain_size {
            let mut x2_0_fe = data.r2_0.clone();
            x2_0_fe.sub(&all_client_s0[idx]);
            let mut x2_1_fe = data.r2_1.clone();
            x2_1_fe.sub(&all_client_s1[idx]);
            let mut x2_fe = x2_0_fe.clone(); 
            x2_fe.add(&x2_1_fe);

            let p_minus1 = (FE::zero() - FE::one()).value();
            let p = p_minus1 + 1;
            let half_p = p / 2;
            let raw = x2_fe.value();
            let signed = if raw > half_p {
                (raw as i128) - (p as i128)
            } else {
                raw as i128
            };
            let domain_i = data.updated_domain as i128;
            let x2_val = ((signed % domain_i + domain_i) % domain_i) as u64;

            let _x2_opened = FE::from(x2_val as u32);
            
            let alpha_x2_0 = data.alpha_r2_0.clone() - all_client_m0[idx].clone();
            let alpha_x2_1 = data.alpha_r2_1.clone() - all_client_m1[idx].clone();
            
            let mut z2_0 = x2_fe;
            z2_0.mul(&data.alpha_val_0);
            z2_0.sub(&alpha_x2_0);
            
            let mut z2_1 = x2_fe;
            z2_1.mul(&data.alpha_val_1);
            z2_1.sub(&alpha_x2_1);
            
            let z2_opened = z2_0 + z2_1;
            if z2_opened.value() != 0 {
                panic!("MAC failure on r2-col_sum opening for idx {}", idx);
            }

            let mut col_sum_shifted_val_1_0 = vec![FE::zero(); data.updated_domain];
            let mut col_sum_shifted_val_1_1 = vec![FE::zero(); data.updated_domain];
            let mut col_sum_shifted_val_2_0 = vec![FE::zero(); data.updated_domain];
            let mut col_sum_shifted_val_2_1 = vec![FE::zero(); data.updated_domain];
            
            for i in 0..data.updated_domain {
                let shift_idx = (i + x2_val as usize) % data.updated_domain;
                col_sum_shifted_val_1_0[i] = data.col_sum_values1_0[shift_idx].clone();
                col_sum_shifted_val_1_1[i] = data.col_sum_values1_1[shift_idx].clone();
                col_sum_shifted_val_2_0[i] = data.col_sum_values2_0[shift_idx].clone();
                col_sum_shifted_val_2_1[i] = data.col_sum_values2_1[shift_idx].clone();
            }
            
            all_col_shifted_values.push((col_sum_shifted_val_1_0, col_sum_shifted_val_1_1, 
                                        col_sum_shifted_val_2_0, col_sum_shifted_val_2_1));
        }

        let mut threshold_sum_0 = FE::zero();
        let mut threshold_sum_1 = FE::zero();
        let mut threshold_mac_0 = FE::zero();
        let mut threshold_mac_1 = FE::zero();
        
        for idx in 0..data.domain_size {
            let (ref col_1_0, ref col_1_1, ref col_2_0, ref col_2_1) = &all_col_shifted_values[idx];
            threshold_sum_0.add(&col_1_0[current_threshold]);
            threshold_sum_1.add(&col_1_1[current_threshold]);
            threshold_mac_0.add(&col_2_0[current_threshold]);
            threshold_mac_1.add(&col_2_1[current_threshold]);
        }
        let _threshold_sum = threshold_sum_0.clone() + threshold_sum_1.clone();
 
        let r3_shift_0 = data.r3_0.clone() - threshold_sum_0.clone();
        let r3_shift_1 = data.r3_1.clone() - threshold_sum_1.clone();
        let r3_opened = r3_shift_0 + r3_shift_1;
        
        let p_minus1 = (FE::zero() - FE::one()).value();
        let p = p_minus1 + 1;
        let half_p = p / 2;
        let raw = r3_opened.value();
        let signed = if raw > half_p {
            (raw as i128) - (p as i128)
        } else {
            raw as i128
        };
        let domain_i = data.max_possible_sum as i128;
        let r3_shift_val = ((signed % domain_i + domain_i) % domain_i) as u64;

        let r3_threshold_opened = FE::from(r3_shift_val as u32);
        let alpha_r3_threshold_0 = data.alpha_r3_0.clone() - threshold_mac_0.clone();
        let alpha_r3_threshold_1 = data.alpha_r3_1.clone() - threshold_mac_1.clone();
        
        let mut z3_0 = r3_threshold_opened.clone();
        z3_0.mul(&data.alpha_val_0);
        z3_0.sub(&alpha_r3_threshold_0);
        
        let mut z3_1 = r3_threshold_opened.clone();
        z3_1.mul(&data.alpha_val_1);
        z3_1.sub(&alpha_r3_threshold_1);
        
        let _z3_total = z3_0 + z3_1;

        let mut tie_shifted_val_1_0 = vec![FE::zero(); data.max_possible_sum];
        let mut tie_shifted_val_1_1 = vec![FE::zero(); data.max_possible_sum];
        let mut tie_shifted_val_2_0 = vec![FE::zero(); data.max_possible_sum];
        let mut tie_shifted_val_2_1 = vec![FE::zero(); data.max_possible_sum];
        
        for i in 0..data.max_possible_sum {
            let shift_idx = (i + r3_shift_val as usize) % data.max_possible_sum;
            tie_shifted_val_1_0[i] = data.tie_values1_0[shift_idx].clone();
            tie_shifted_val_1_1[i] = data.tie_values1_1[shift_idx].clone();
            tie_shifted_val_2_0[i] = data.tie_values2_0[shift_idx].clone();
            tie_shifted_val_2_1[i] = data.tie_values2_1[shift_idx].clone();
        }

        let mut exact_one_check = FE::zero();
        exact_one_check.add(&tie_shifted_val_1_0[0]);
        exact_one_check.add(&tie_shifted_val_1_1[0]);
        
        if exact_one_check.value() == 0 {
            second_highest_found = true;
            
            let mut z_0 = exact_one_check.clone();
            z_0.mul(&data.alpha_val_0);
            z_0.sub(&tie_shifted_val_2_0[1]);

            let mut z_1 = exact_one_check.clone();
            z_1.mul(&data.alpha_val_1);
            z_1.sub(&tie_shifted_val_2_1[1]);

            let _z_total = z_0 + z_1;

            // YOUR EXACT PROTOCOL SECTION - PRESERVED COMPLETELY
            for idx in 0..data.domain_size {
                let (ref col_1_0, ref col_1_1, ref col_2_0, ref col_2_1) = &all_col_shifted_values[idx];
                
                // Sum from current_threshold to n-1 to check >= current_threshold for this specific bid level
                let mut col_ge_threshold = FE::zero();
                let mut col_ge_threshold_mac = FE::zero();
                
                for j in current_threshold..data.updated_domain {
                    col_ge_threshold.add(&col_1_0[j]);
                    col_ge_threshold.add(&col_1_1[j]);
                    col_ge_threshold_mac.add(&col_2_0[j]);
                    col_ge_threshold_mac.add(&col_2_1[j]);
                }
                
                if col_ge_threshold.value() >= 1 {
                    second_highest_bid = idx;
                    
                    // MAC check for this specific bid level
                    let mut mac_accum_0 = FE::zero();
                    let mut mac_accum_1 = FE::zero();
                    
                    for j in current_threshold..data.updated_domain {
                        mac_accum_0.add(&col_2_0[j]);
                        mac_accum_1.add(&col_2_1[j]);
                    }

                    let mut z_0 = col_ge_threshold.clone();
                    z_0.mul(&data.alpha_val_0);
                    z_0.sub(&mac_accum_0);

                    let mut z_1 = col_ge_threshold.clone();
                    z_1.mul(&data.alpha_val_1);
                    z_1.sub(&mac_accum_1);

                    let z_total = z_0 + z_1;
                    if z_total.value() != 0 {
                        // panic!("MAC failure on second-highest reveal!");
                    }

                    if current_threshold < data.num_clients - 1 {
                        // println!("TIE DETECTED: {} bidders tied for highest bid", data.num_clients - current_threshold);
                    }
                    // println!("The value of second highest bid is: {}", second_highest_bid);

                    // Find highest bidder
                    for bidder in 0..data.num_clients {
                        let mut temp_sum = FE::zero();
                        let mut temp_sum_mac = FE::zero();
                        for index in 0..=idx {
                            let sh_index = (index + data.x_val[bidder] as usize) % data.domain_size;
                            temp_sum.add(&data.values1_0[sh_index].clone());
                            temp_sum.add(&data.values1_1[sh_index].clone());
                            temp_sum_mac.add(&data.values2_0[sh_index].clone());
                            temp_sum_mac.add(&data.values2_1[sh_index].clone());
                        }

                        let mut expected_mac = temp_sum.clone();
                        expected_mac.mul(&data.alpha_val);
                        if temp_sum_mac.value() != expected_mac.value() {
                            panic!("MAC failure on highest-bidder reveal for bidder {}", bidder);
                        }
                        
                        if temp_sum.value() == 0 {
                            // println!("The index of highest bidder is: {}", bidder);
                        }
                    }
                    break;
                }
            }
            break;
        }
        
        if !second_highest_found {
            current_threshold -= 1;
        }
    }

    // End timing the online phase
    let online_duration = client_start.elapsed();
    online_duration.as_secs_f64() * 1000.0
}

fn run_throughput_test(data: &AuctionData, target_aps: u32, duration_secs: u64) -> BenchmarkResult {
    println!("Testing {} aps for {} seconds...", target_aps, duration_secs);
    
    // Pin to CPU core 0 (Linux only)
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let pid = std::process::id();
        let _ = Command::new("taskset")
            .args(&["-cp", "0", &pid.to_string()])
            .output();
    }

    let (tx, rx) = mpsc::sync_channel::<(u64, Instant)>(1000);
    let mut latencies = Vec::new();
    
    // Request generator thread
    let generator_handle = thread::spawn(move || {
        let interval = std::time::Duration::from_secs_f64(1.0 / target_aps as f64);
        let end_time = Instant::now() + std::time::Duration::from_secs(duration_secs);
        let mut request_id = 0;
        
        while Instant::now() < end_time {
            let arrival_time = Instant::now();
            
            match tx.try_send((request_id, arrival_time)) {
                Ok(_) => {
                    request_id += 1;
                },
                Err(mpsc::TrySendError::Full(_)) => {
                    // Channel full, skip request (system overloaded)
                },
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    break;
                }
            }
            
            thread::sleep(interval);
        }
    });

    // Process requests sequentially (creates queuing effects)
    let start_time = Instant::now();
    let mut processed = 0;
    let mut failed = 0;
    
    while start_time.elapsed().as_secs() < duration_secs {
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok((_request_id, arrival_time)) => {
                // Run the complete auction protocol
                match std::panic::catch_unwind(|| run_auction_protocol(&data)) {
                    Ok(_auction_latency) => {
                        let processing_end = Instant::now();
                        let total_latency = processing_end.duration_since(arrival_time).as_secs_f64() * 1000.0;
                        latencies.push(total_latency);
                        processed += 1;
                    },
                    Err(_) => {
                        failed += 1;
                    }
                }
                
                if processed % 50 == 0 && processed > 0 {
                    let elapsed = start_time.elapsed().as_secs();
                    let current_rate = processed as f64 / elapsed as f64;
                    println!("  Progress: {} processed ({} failed) - {:.1} aps", 
                           processed, failed, current_rate);
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    generator_handle.join().unwrap();
    
    // Calculate statistics
    if latencies.is_empty() {
        return BenchmarkResult {
            target_aps,
            actual_aps: 0.0,
            p50_ms: 0.0,
            p99_ms: 0.0,
            samples: 0,
            mean_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
        };
    }
    
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let p50 = latencies[latencies.len() * 50 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];
    let mean = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let actual_aps = latencies.len() as f64 / duration_secs as f64;
    
    BenchmarkResult {
        target_aps,
        actual_aps,
        p50_ms: p50,
        p99_ms: p99,
        samples: latencies.len(),
        mean_ms: mean,
        min_ms: min,
        max_ms: max,
    }
}

fn save_results(results: &[BenchmarkResult], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(filename)?;
    
    writeln!(file, "target_aps,actual_aps,p50_ms,p99_ms,samples,mean_ms,min_ms,max_ms")?;
    
    for result in results {
        writeln!(file, "{},{:.2},{:.2},{:.2},{},{:.2},{:.2},{:.2}",
                result.target_aps,
                result.actual_aps,
                result.p50_ms,
                result.p99_ms,
                result.samples,
                result.mean_ms,
                result.min_ms,
                result.max_ms)?;
    }
    
    Ok(())
}

fn run_benchmark_suite(data: &AuctionData) -> Result<(), Box<dyn std::error::Error>> {
    let throughput_values: Vec<u32> = (350..=390).step_by(10).collect(); // [100, 150, 200, 250, 300, 350, 400, 450, 500]
    let test_duration = 10; // seconds per test
    let mut results = Vec::new();
    
    println!("🚀 Starting automated benchmark suite");
    println!("Testing throughput values: {:?}", throughput_values);
    println!("Test duration: {} seconds per value\n", test_duration);
    
    for (i, &target_aps) in throughput_values.iter().enumerate() {
        println!("▶️  Test {}/{}: {} auctions/second", i + 1, throughput_values.len(), target_aps);
        
        let result = run_throughput_test(data, target_aps, test_duration);
        
        println!("   Results: {:.1} actual aps, P50={:.1}ms, P99={:.1}ms, {} samples\n",
                result.actual_aps, result.p50_ms, result.p99_ms, result.samples);
        
        results.push(result);
        
        // Brief pause between tests
        if i < throughput_values.len() - 1 {
            println!("   Cooling down for 3 seconds...\n");
            thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    
    // Save results to CSV
    save_results(&results, "benchmark_results.csv")?;
    
    // Print summary table
    println!("🎯 BENCHMARK SUMMARY");
    println!("==========================================");
    println!("{:<10} {:<12} {:<10} {:<10} {:<8} {:<10}", 
             "Target", "Actual", "P50 (ms)", "P99 (ms)", "Samples", "Efficiency");
    println!("------------------------------------------");
    
    for result in &results {
        let efficiency = (result.actual_aps / result.target_aps as f64) * 100.0;
        println!("{:<10} {:<12.1} {:<10.1} {:<10.1} {:<8} {:<10.1}%",
                result.target_aps,
                result.actual_aps,
                result.p50_ms,
                result.p99_ms,
                result.samples,
                efficiency);
    }
    
    println!("\n📊 Results saved to: benchmark_results.csv");
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // Load preprocessed data
    let filename = if args.len() > 1 && !args[1].parse::<f64>().is_ok() && args[1] != "--benchmark" {
        args[1].clone()
    } else {
        "auction_data.bin".to_string()
    };
    
    let mut file = File::open(&filename)?;
    let mut encoded = Vec::new();
    file.read_to_end(&mut encoded)?;
    let data: AuctionData = bincode::deserialize(&encoded)?;
    
    // Check for benchmark mode
    if args.len() == 1 || (args.len() > 1 && args[1] == "--benchmark") {
        // Automatic benchmark suite mode (100-500 aps)
        return run_benchmark_suite(&data);
    }
    
    // Check if this is manual server mode
    if args.len() >= 2 {
        if let Ok(target_aps) = args[1].parse::<f64>() {
            // Manual server mode for single throughput testing
            let duration_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
            
            let result = run_throughput_test(&data, target_aps as u32, duration_secs);
            
            println!("Manual test results:");
            println!("Target: {} aps", target_aps);
            println!("Actual: {:.2} aps", result.actual_aps);
            println!("P50: {:.2}ms", result.p50_ms);
            println!("P99: {:.2}ms", result.p99_ms);
            println!("Samples: {}", result.samples);
            
            return Ok(());
        }
    }
    
    // Single auction mode (original behavior)
    let latency = run_auction_protocol(&data);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("latencies.txt")?;
    
    writeln!(file, "{:.6}", latency)?;
    
    Ok(())
}
