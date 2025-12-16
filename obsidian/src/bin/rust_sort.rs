use std::time::Instant;
use std::fs::File;
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::sync::mpsc;
use std::thread;
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BidData {
    pub num_inputs: usize,
    pub domain_size: usize,
    pub bids: Vec<u32>,
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

fn run_native_sort_on_bids(bid_data: &BidData) -> f64 {
    // Start timing the online sorting phase
    let sort_start = Instant::now();
    
    // Clone the bids for sorting (to preserve original data)
    let mut bids = bid_data.bids.clone();
    
    // Sort the bids to find top 2
    bids.sort_unstable();
    
    // Find highest and second highest
    let highest = bids[bids.len() - 1];
    let second_highest = if bids.len() > 1 {
        bids[bids.len() - 2]
    } else {
        0
    };
    
    // Optional verification (minimal overhead)
    let _verification = highest >= second_highest;
    
    // End timing the online sorting phase
    let sort_duration = sort_start.elapsed();
    sort_duration.as_secs_f64() * 1000.0
}

fn run_throughput_test(bid_data: &BidData, target_aps: u32, duration_secs: u64) -> BenchmarkResult {
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
    
    while start_time.elapsed().as_secs() < duration_secs {
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok((_request_id, arrival_time)) => {
                let _sort_latency = run_native_sort_on_bids(&bid_data);
                let processing_end = Instant::now();
                let total_latency = processing_end.duration_since(arrival_time).as_secs_f64() * 1000.0;
                latencies.push(total_latency);
                processed += 1;
                
                if processed % 10000 == 0 && processed > 0 {
                    let elapsed = start_time.elapsed().as_secs();
                    let current_rate = processed as f64 / elapsed as f64;
                    println!("  Progress: {} processed - {:.1} aps", processed, current_rate);
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

fn run_benchmark_suite(bid_data: &BidData) -> Result<(), Box<dyn std::error::Error>> {
    let throughput_values: Vec<u32> = (10000..=50000).step_by(5000).collect(); // [100, 150, 200, 250, 300, 350, 400, 450, 500]
    let test_duration = 10; // seconds per test
    let mut results = Vec::new();
    
    println!("🚀 Starting native sort throughput benchmark");
    println!("Using {} pre-generated bids from domain [0, {}]", bid_data.num_inputs, bid_data.domain_size - 1);
    println!("Testing throughput values: {:?}", throughput_values);
    println!("Test duration: {} seconds per value\n", test_duration);
    
    for (i, &target_aps) in throughput_values.iter().enumerate() {
        println!("▶️  Test {}/{}: {} auctions/second", i + 1, throughput_values.len(), target_aps);
        
        let result = run_throughput_test(bid_data, target_aps, test_duration);
        
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
    save_results(&results, "native_sort_benchmark.csv")?;
    
    // Print summary table
    println!("🎯 NATIVE SORT BENCHMARK SUMMARY");
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
    
    println!("\n📊 Results saved to: native_sort_benchmark.csv");
    
    // Find key insights
    let max_efficient = results.iter()
        .filter(|r| (r.actual_aps / r.target_aps as f64) >= 0.9)
        .max_by(|a, b| a.actual_aps.partial_cmp(&b.actual_aps).unwrap());
    
    let reasonable_latency = results.iter()
        .filter(|r| r.p99_ms < 10.0)
        .max_by(|a, b| a.actual_aps.partial_cmp(&b.actual_aps).unwrap());
    
    println!("\n🎯 KEY INSIGHTS:");
    if let Some(result) = max_efficient {
        println!("   Max efficient throughput (≥90%): {:.1} aps at {} target", 
                result.actual_aps, result.target_aps);
    }
    if let Some(result) = reasonable_latency {
        println!("   Max throughput with P99 < 10ms: {:.1} aps at {} target", 
                result.actual_aps, result.target_aps);
    }
    
    let peak = results.iter().max_by(|a, b| a.actual_aps.partial_cmp(&b.actual_aps).unwrap()).unwrap();
    println!("   Peak throughput achieved: {:.1} aps at {} target", peak.actual_aps, peak.target_aps);
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // Load preprocessed bid data
    let filename = if args.len() > 1 && !args[1].parse::<f64>().is_ok() && args[1] != "--benchmark" {
        args[1].clone()
    } else {
        "bid_data.bin".to_string()
    };
    
    let mut file = File::open(&filename)?;
    let mut encoded = Vec::new();
    file.read_to_end(&mut encoded)?;
    let bid_data: BidData = bincode::deserialize(&encoded)?;
    
    // Check for benchmark mode
    if args.len() == 1 || (args.len() > 1 && args[1] == "--benchmark") {
        // Automatic benchmark suite mode (100-500 aps)
        return run_benchmark_suite(&bid_data);
    }
    
    // Check if this is manual server mode
    if args.len() >= 2 {
        if let Ok(target_aps) = args[1].parse::<f64>() {
            // Manual server mode for single throughput testing
            let duration_secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30);
            
            let result = run_throughput_test(&bid_data, target_aps as u32, duration_secs);
            
            println!("Manual test results:");
            println!("Target: {} aps", target_aps);
            println!("Actual: {:.2} aps", result.actual_aps);
            println!("P50: {:.2}ms", result.p50_ms);
            println!("P99: {:.2}ms", result.p99_ms);
            println!("Samples: {}", result.samples);
            
            return Ok(());
        }
    }
    
    // Single sort mode (original behavior)
    let latency = run_native_sort_on_bids(&bid_data);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("sort_latencies.txt")?;
    
    writeln!(file, "{:.6}", latency)?;
    
    Ok(())
}
