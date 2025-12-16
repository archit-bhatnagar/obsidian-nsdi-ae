// use counttree::dpf::*;
use std::time::{Duration, Instant};
use std::thread::sleep;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*; // Import the sketch module
// use counttree::mpc;
use rand::Rng;
use counttree::prg::FromRng; // Import the FromRng trait
// use num_bigint::BigUint;

/// Simple function to simulate network latency (RTT = 20ms)
fn simulate_network_round() -> Duration {
    let rtt = Duration::from_millis(20);
    sleep(rtt); // Simulate network delay
    rtt
}

/// Track communication rounds and network latency
struct NetworkStats {
    rounds: u32,
    total_latency: Duration,
}

impl NetworkStats {
    fn new() -> Self {
        Self {
            rounds: 0,
            total_latency: Duration::new(0, 0),
        }
    }
    
    fn add_round(&mut self) {
        self.rounds += 1;
        let latency = simulate_network_round();
        self.total_latency += latency;
    }
}

/// Generate Beaver triples for secure multiplication in preprocessing
fn generate_beaver_triples(num_needed: usize) -> Vec<(FE, FE, FE, FE, FE, FE)> {
    let mut triples = Vec::with_capacity(num_needed);
    let mut rng = rand::thread_rng();
    
    for _ in 0..num_needed {
        // Generate random a, b
        let mut a = FE::zero();
        a.from_rng(&mut rng);
        let mut b = FE::zero();
        b.from_rng(&mut rng);
        // Compute c = a*b
        let c: FE = a.clone() * b.clone();
        
        // Generate shares
        let (a0, a1) = generate_alpha_shares(&a);
        let (b0, b1) = generate_alpha_shares(&b);
        let (c0, c1) = generate_alpha_shares(&c);
        
        triples.push((a0, a1, b0, b1, c0, c1));
    }
    
    triples
}

/// Generates a one‑hot encoded vector in conventional (MSB‑first) order
/// while keeping track of the underlying MSB-first "hot" index.
/// In this representation, the printed string is as you usually see it (left: MSB, right: LSB),
/// but the random index generated (lsb_hot_index) is directly usable for DPF key generation.
fn generate_one_hot_conventional(length: usize) -> (Vec<bool>, usize) {
    let mut rng = rand::thread_rng();
    // Generate a random hot index in MSB-first order.
    let lsb_hot_index = rng.gen_range(0, length);
    let mut bits = vec![false; length];
    // Compute the corresponding index in conventional (MSB‑first) order.
    let msb_index = length - 1 - lsb_hot_index;
    bits[msb_index] = true;
    (bits, lsb_hot_index)
}

/// Generates two random field element shares that sum to alpha_val
fn generate_alpha_shares<T: prg::FromRng + Clone + Group>(alpha_val: &T) -> (T, T) {
    let mut share1 = T::zero();
    share1.randomize();
    let mut share2 = alpha_val.clone();
    share2.sub(&share1);
    (share1, share2)
}

/// Pre-processes and generates DPF keys for MAC computation
fn preprocess_mac(
    domain_size: usize,
    alpha_val: &FE,
) -> ((SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), (SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), FE) {
    // Generate random position r as a usize
    let mut rng = rand::thread_rng();
    let r_usize = rng.gen_range(0, domain_size);
    println!("Random position r (usize): {}", r_usize);

    let nbits = (domain_size as f64).log2().ceil() as u8;

    // Convert r to FE
    let r: FE = FE::from(r_usize as u32); // Assuming FE implements From<u32>
    println!("Random position r (FE): {:?}", r);

    let alpha = u32_to_bits(nbits, r_usize as u32);
    println!("Alpha: {:?}", alpha);
    let betas = vec![FE::one(); alpha.len() - 1];
    let beta_last = FE::one();
    let key_pair1 = SketchDPFKey::gen(&alpha, &betas, &beta_last);

    // Generate MAC DPF key pair (alpha at position r, 0 elsewhere)
    let betas2 = vec![FE::one(); alpha.len() - 1];
    // let beta_last2 = FE::one();
    let beta_last2 = alpha_val.clone();
    let key_pair2 = SketchDPFKey::gen(&alpha, &betas2, &beta_last2);

    (key_pair1.into(), key_pair2.into(), r)
}

/// Evaluates the SketchDPFKey for all values in the domain and returns the results as vectors.
fn eval_all(key: &SketchDPFKey<FE, FE>, domain_size: usize) -> Vec<FE> {
    let mut all_values = Vec::with_capacity(domain_size);
    // let last_values: Vec<FE> = Vec::with_capacity(domain_size);
    let nbits = (domain_size as f64).log2().ceil() as u8;

    for i in 0..domain_size {
        let bits = u32_to_bits(nbits, i as u32); // For a domain of 64 it is 6 bits
        let value = key.eval(&bits);
        // println!("Value: {:?}", value.clone());
        all_values.push(value.clone());
        // last_values.push(last.clone());
    }

    all_values
}

fn mal_preprocess_check(
    values1_0: &[FE], values1_1: &[FE],
    values2_0: &[FE], values2_1: &[FE],
    domain_size: usize,
    r: &FE,
    alpha_val: &FE,) -> (FE, FE) {
    // Generate shares of r
    let (r0, r1) = generate_alpha_shares(r);
    // Linear sketch randomness
    let mut rng1 = rand::thread_rng();
    let mut rng2 = rand::thread_rng();
    let a1: Vec<FE> = (0..domain_size).map(|_| { let mut f=FE::zero(); f.from_rng(&mut rng1); f }).collect();
    let a2: Vec<FE> = (0..domain_size).map(|_| { let mut f=FE::zero(); f.from_rng(&mut rng2); f }).collect();
    let a3: Vec<FE> = a1.iter().zip(a2.iter()).map(|(x,y)| *x * *y).collect();
    let a4: Vec<FE> = (0..domain_size).map(|i| FE::from(i as u32)).collect();

    // Inner products
    let z1_0: FE = values1_0.iter().zip(a1.iter()).map(|(v,a)| *v * *a).sum();
    let z2_0: FE = values1_0.iter().zip(a2.iter()).map(|(v,a)| *v * *a).sum();
    let z3_0: FE = values1_0.iter().zip(a3.iter()).map(|(v,a)| *v * *a).sum();
    let z4_0: FE = values1_0.iter().zip(a4.iter()).map(|(v,a)| *v * *a).sum();
    let z_star_0: FE = values2_0.iter().zip(a1.iter()).map(|(v,a)| *v * *a).sum();
    let z1_1: FE = values1_1.iter().zip(a1.iter()).map(|(v,a)| *v * *a).sum();
    let z2_1: FE = values1_1.iter().zip(a2.iter()).map(|(v,a)| *v * *a).sum();
    let z3_1: FE = values1_1.iter().zip(a3.iter()).map(|(v,a)| *v * *a).sum();
    let z4_1: FE = values1_1.iter().zip(a4.iter()).map(|(v,a)| *v * *a).sum();
    let z_star_1: FE = values2_1.iter().zip(a1.iter()).map(|(v,a)| *v * *a).sum();
    let z1 = z1_0 + z1_1;
    let z3 = z3_0 + z3_1;
    let z_star = z_star_0 + z_star_1;

    // Beaver triplets
    let mut rng = rand::thread_rng();
    let mut a_b = FE::zero(); a_b.from_rng(&mut rng);
    let mut b_b = FE::zero(); b_b.from_rng(&mut rng);
    let c_b: FE = a_b * b_b;
    let (a0,a1) = generate_alpha_shares(&a_b);
    let (b0,b1) = generate_alpha_shares(&b_b);
    let (c0,c1) = generate_alpha_shares(&c_b);

    // Compute z1*z2
    let e0 = z1_0 - a0.clone(); let f0 = z2_0 - b0.clone();
    let e1 = z1_1 - a1.clone(); let f1 = z2_1 - b1.clone();
    let comb_e = e0 + e1; let comb_f = f0 + f1;
    let z1z2_0 = comb_e.clone()*b0.clone() + comb_f.clone()*a0.clone() + c0.clone();
    let z1z2_1 = comb_e.clone()*b1.clone() + comb_f.clone()*a1.clone() + c1.clone();
    let z1z2 = comb_e*comb_f + z1z2_0 + z1z2_1;

    // z4 - r shares
    let result0 = z4_0 - r0.clone();
    let result1 = z4_1 - r1.clone();
    let sum_z1z2_z3 = z1z2 - z3;
    let sum_z4_r = result0 + result1;
    let final_res = sum_z1z2_z3 + sum_z4_r;
    println!("MAC check result: {:?}", final_res.value());
    // mac check (z_star)
    let mac_check = *alpha_val * z1 - z_star;
    println!("MAC check: {:?}", mac_check.value());

    (r0, r1)
}

// Function to run a single auction process and return the timing results
fn run_auction(num_clients: usize, domain_size: usize) -> (Duration, Duration, NetworkStats) {
    let mut network_stats = NetworkStats::new();
    let overall_start = Instant::now();

    // Part 1: MAC Pre-processing and Evaluation
    println!("\nMAC Pre-processing and Evaluation:");
    
    // Generate a random alpha value using FE::random()
    let alpha_val = FE::random();
    
    // Pre-process and generate DPF keys
    let ((key1_0, key1_1), (key2_0, key2_1), r) = preprocess_mac(domain_size, &alpha_val);
    
    // Evaluate all values for both key pairs
    let values1_0 = eval_all(&key1_0, domain_size);
    let values1_1 = eval_all(&key1_1, domain_size);
    let values2_0 = eval_all(&key2_0, domain_size);
    let values2_1 = eval_all(&key2_1, domain_size);

    // Generate Beaver triples for MAC checks during preprocessing
    // Estimate how many are needed based on operations
    let num_mac_checks_needed = num_clients * 2 + domain_size + 10; // For r-a and r2-a operations
    let beaver_triples_preprocessing_start = Instant::now();
    let beaver_triples = generate_beaver_triples(num_mac_checks_needed);
    let beaver_triples_time = beaver_triples_preprocessing_start.elapsed();
    println!("Beaver triple generation took: {:?}", beaver_triples_time);

    let (r_0, r_1) = mal_preprocess_check(&values1_0, &values1_1, &values2_0, &values2_1, domain_size, &r, &alpha_val);

    let preprocess_time = overall_start.elapsed();
    println!("Pre-processing took: {:?}", preprocess_time);

    let client_start = Instant::now();

    // Shift the DPF evaluation vectors.
    // let mut y = vec![FE::zero(); domain_size];
    // let mut my = vec![FE::zero(); domain_size];
    // for i in 0..domain_size {
    //     let mut combined_y = values1_0[i].clone();
    //     combined_y.add(&values1_1[i]);
    //     y[i] = combined_y;
        
    //     let mut combined_my = values2_0[i].clone();
    //     combined_my.add(&values2_1[i]);
    //     my[i] = combined_my;
    // }
    
    // For each client, generate a different secret input a and run the lookup steps.
    let mut all_client_s0 = vec![FE::zero(); domain_size];
    let mut all_client_s1 = vec![FE::zero(); domain_size];
    let mut all_client_m0 = vec![FE::zero(); domain_size];
    let mut all_client_m1 = vec![FE::zero(); domain_size];
    let mut x_val = vec![0; num_clients];

    for client in 0..num_clients {
        println!("\nClient {}:", client);
        // Generate client's one‑hot input.
        let (one_hot, lsb_hot_index) = generate_one_hot_conventional(domain_size);
        // Convert to conventional index (0 = leftmost).
        let a_index = domain_size - 1 - lsb_hot_index;
        println!("  Secret input index (a): {}", a_index);
        let a_val = FE::from(a_index as u32);

        // (In a real protocol, a would be secret-shared; here we simulate by directly using the value.)
        let (a_0, a_1) = generate_alpha_shares(&a_val);

        // Use the shares of r that were already generated
        let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
        let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);
        
        // COMMUNICATION: Opening x_val[client]
        println!("NETWORK: Party 1 sends x_share1 to Party 0");
        network_stats.add_round(); // Simulate network delay
        
        // open by summing shares:
        x_val[client] = (x_share0 + x_share1) % (domain_size as u64);

        // Add MAC check for the opened value
        let mut x_fe = FE::from(x_val[client] as u32);
        let mut expected_mac = x_fe.clone();
        expected_mac.mul(&alpha_val);

        // Compute MAC shares for x = r - a
        let mut mac_x_0 = r_0.clone();
        mac_x_0.mul(&alpha_val);
        let mut temp_for_mac = a_0.clone();
        temp_for_mac.mul(&alpha_val);
        mac_x_0.sub(&temp_for_mac); 

        let mut mac_x_1 = r_1.clone();
        mac_x_1.mul(&alpha_val);
        temp_for_mac = a_1.clone();
        temp_for_mac.mul(&alpha_val);
        mac_x_1.sub(&temp_for_mac); 

        // COMMUNICATION: MAC check for opened value
        println!("NETWORK: Parties perform MAC check on x_val");
        network_stats.add_round(); // Simulate network delay
        
        let mac_x = mac_x_0 + mac_x_1;

        println!("Opened value x = r - a: {}", x_val[client]);

        // Shift the values1_0, values1_1, values2_0, values2_1 by x_val[client]
        let mut shifted_val_1_0 = vec![FE::zero(); domain_size];
        let mut shifted_val_1_1 = vec![FE::zero(); domain_size];
        let mut shifted_val_2_0 = vec![FE::zero(); domain_size];
        let mut shifted_val_2_1 = vec![FE::zero(); domain_size];
        for i in 0..domain_size {
            let idx = (i + x_val[client] as usize) % domain_size;
            shifted_val_1_0[i] = values1_0[idx].clone();
            shifted_val_1_1[i] = values1_1[idx].clone();
            // MAC
            shifted_val_2_0[i] = values2_0[idx].clone();
            shifted_val_2_1[i] = values2_1[idx].clone();
        }

        // Compute cumulative sum of shifted_values
        let mut cumulative_s0 = FE::zero();
        let mut cumulative_s1 = FE::zero();
        let mut cumulative_m0 = FE::zero();
        let mut cumulative_m1 = FE::zero();
        for i in 0..domain_size {
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

    // Now convert the column sum shares into their corresponding DPF values
    // do another round of generating a random r and shifting the values
    // IMP: the domain size here becomes equal to the number of clients + 1 actually (range for the sum to be)
    // this is because the column sum can be anywhere from 0 to num_clients which is (num_clients + 1) values
    let updated_domain = num_clients + 1;
    let ((key1_0, key1_1), (key2_0, key2_1), r2) = preprocess_mac(updated_domain, &alpha_val);
    println!("r2 is: {}", r2);

    // Evaluate all values for both key pairs
    let col_sum_values1_0 = eval_all(&key1_0, updated_domain);
    let col_sum_values1_1 = eval_all(&key1_1, updated_domain);
    let col_sum_values2_0 = eval_all(&key2_0, updated_domain);
    let col_sum_values2_1 = eval_all(&key2_1, updated_domain);
    
    // Split r2 into additive shares for secure opening
    let (r2_0, r2_1) = mal_preprocess_check(&col_sum_values1_0, &col_sum_values1_1, &col_sum_values2_0, &col_sum_values2_1, updated_domain, &r2, &alpha_val);

    let mut temp_sum = FE::zero();
    let mut temp_sum_mac = FE::zero();
    // now for each of the columns do the shifting to make sure the lookup happens
    // for all the columns
    for idx in 0..domain_size {
        
        // Compute each party's FE‐share of x2 = r2 – col_sum:
        let mut x2_0_fe = r2_0.clone();
        x2_0_fe.sub(&all_client_s0[idx]);    // field subtraction
        let mut x2_1_fe = r2_1.clone();
        x2_1_fe.sub(&all_client_s1[idx]);    // field subtraction
        
        // COMMUNICATION: Opening x2_val
        println!("NETWORK: Party 1 sends x2_1_fe to Party 0 for column {}", idx);
        network_stats.add_round(); // Simulate network delay
        
        let mut x2_fe = x2_0_fe.clone(); 
        x2_fe.add(&x2_1_fe);
        // pull out "p–1" via (0 - 1) in FE, then compute p = (p–1) + 1:
        let p_minus1 = (FE::zero() - FE::one()).value();       // = p - 1
        let p = p_minus1 + 1;                                  // = p
        let half_p = p / 2;

        // map raw field value into [−p/2 … +p/2):
        let raw = x2_fe.value();
        let signed = if raw > half_p {
            (raw as i128) - (p as i128)
        } else {
            raw as i128
        };
        // reduce into your small-domain [0..updated_domain):
        let domain_i = updated_domain as i128;
        let x2_val = ((signed % domain_i + domain_i) % domain_i) as u64;
        println!("Opened value x = r2 - col_sum: {}", x2_val);
        
        // COMMUNICATION: MAC check for x2_val
        println!("NETWORK: Parties perform MAC check on x2_val");
        network_stats.add_round(); // Simulate network delay

        let mut col_sum_shifted_val_1_0 = vec![FE::zero(); updated_domain];
        let mut col_sum_shifted_val_1_1 = vec![FE::zero(); updated_domain];
        let mut col_sum_shifted_val_2_0 = vec![FE::zero(); updated_domain];
        let mut col_sum_shifted_val_2_1 = vec![FE::zero(); updated_domain];
        // can only do this for the last few 
        for i in 0..updated_domain {
            let shift_idx = (i + x2_val as usize) % updated_domain;
            col_sum_shifted_val_1_0[i] = col_sum_values1_0[shift_idx].clone();
            col_sum_shifted_val_1_1[i] = col_sum_values1_1[shift_idx].clone();
            col_sum_shifted_val_2_0[i] = col_sum_values2_0[shift_idx].clone();
            col_sum_shifted_val_2_1[i] = col_sum_values2_1[shift_idx].clone();    
        }

        // now we want to do a comparision >= n - 1 (so just sum up the shares for n-1 and n)
        let mut col_sum_s= FE::zero();
        let mut col_sum_m= FE::zero();
        let n1 = updated_domain - 1;
        let n2 = updated_domain - 2;
        for &j in &[n2, n1] {
            col_sum_s.add(&col_sum_shifted_val_1_0[j]);
            col_sum_s.add(&col_sum_shifted_val_1_1[j]);
            col_sum_m.add(&col_sum_shifted_val_2_0[j]);
            col_sum_m.add(&col_sum_shifted_val_2_1[j]);
        }
        
        // COMMUNICATION: Opening column comparison result
        println!("NETWORK: Opening column comparison result");
        network_stats.add_round(); // Simulate network delay
        
        if col_sum_s.value() >= 1 { // for the tie case
            // COMMUNICATION: MAC check for second highest bid
            println!("NETWORK: MAC verification for second highest bid");
            network_stats.add_round(); // Simulate network delay
            
            // MAC check here
            let mut mac_accum = FE::zero();
            for &j in &[n2, n1] {
                mac_accum.add(&col_sum_shifted_val_2_0[j]);
                mac_accum.add(&col_sum_shifted_val_2_1[j]);
            }
            let mut y_accum = FE::zero();
            for &j in &[n2, n1] {
                y_accum.add(&col_sum_shifted_val_1_0[j]);
                y_accum.add(&col_sum_shifted_val_1_1[j]);
            }
            let mut expect = y_accum.clone();
            expect.mul(&alpha_val);
            if mac_accum.value() != expect.value() {
                panic!("MAC failure on second-highest reveal!");
            }
            // this is the second highest bid, REVEAL
            println!(" The value of second highest bid is: {}", idx);

            // Getting the highest bidder
            for bidder in 0..num_clients {
                temp_sum = FE::zero();
                temp_sum_mac = FE::zero();
                for index in 0..=idx {
                    let sh_index = (index + x_val[bidder] as usize) % domain_size;
                    temp_sum.add(&values1_0[sh_index].clone());
                    temp_sum.add(&values1_1[sh_index].clone());
                    temp_sum_mac.add(&values2_0[sh_index].clone());
                    temp_sum_mac.add(&values2_1[sh_index].clone());
                }

                // COMMUNICATION: Opening highest bidder check
                println!("NETWORK: Opening highest bidder check for bidder {}", bidder);
                network_stats.add_round(); // Simulate network delay
                
                // **MAC check on highest-bidder reveal**
                let mut expected_mac = temp_sum.clone();
                expected_mac.mul(&alpha_val);
                if temp_sum_mac.value() != expected_mac.value() {
                    panic!("MAC failure on highest-bidder reveal for bidder {}", bidder);
                }
                let high_bid_idx: u64 = temp_sum.value();
                
                if high_bid_idx == 0 {
                    // COMMUNICATION: MAC check for highest bidder
                    println!("NETWORK: MAC verification for highest bidder");
                    network_stats.add_round(); // Simulate network delay
                    
                    println!(" The index of highest bidder is: {}", bidder);
                }
            }
            break;
        }
    }

    let client_duration = client_start.elapsed();
    println!("Online time: {:?}", client_duration);
    
    (preprocess_time, client_duration, network_stats)
}

fn main() {
    let num_clients = 100;
    let domain_size = 256;
    let num_runs = 5;
    
    let mut total_preprocess_time = Duration::new(0, 0);
    let mut total_online_time = Duration::new(0, 0);
    let mut total_network_time = Duration::new(0, 0);
    let mut total_network_rounds = 0;
    
    println!("Running auction with {} clients and domain size {} for {} times", num_clients, domain_size, num_runs);
    println!("Using network RTT of 20ms");
    
    for run in 0..num_runs {
        println!("\n=== Run {} ===", run + 1);
        let (preprocess_time, online_time, network_stats) = run_auction(num_clients, domain_size);
        
        total_preprocess_time += preprocess_time;
        total_online_time += online_time;
        total_network_time += network_stats.total_latency;
        total_network_rounds += network_stats.rounds;
        
        println!("  Network rounds: {}", network_stats.rounds);
        println!("  Network time: {:?}", network_stats.total_latency);
        println!("  Computation time: {:?}", online_time - network_stats.total_latency);
    }
    
    // Calculate averages
    let avg_preprocess_time = total_preprocess_time / num_runs as u32;
    let avg_online_time = total_online_time / num_runs as u32;
    let avg_network_time = total_network_time / num_runs as u32;
    let avg_computation_time = avg_online_time - avg_network_time;
    let avg_network_rounds = total_network_rounds / num_runs as u32;
    
    println!("\n=== Average Results ===");
    println!("Average pre-processing time: {:?}", avg_preprocess_time);
    println!("Average online time (total): {:?}", avg_online_time);
    println!("Average network time: {:?}", avg_network_time);
    println!("Average computation time: {:?}", avg_computation_time);
    println!("Average network rounds: {}", avg_network_rounds);
    println!("Network overhead: {:.2}%", (avg_network_time.as_millis() as f64 / avg_online_time.as_millis() as f64) * 100.0);
}
