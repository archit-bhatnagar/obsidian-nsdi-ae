// use counttree::dpf::*;
use std::time::Instant;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*; // Import the sketch module
use rand::Rng;
use counttree::prg::FromRng; // Import the FromRng trait
// use num_bigint::BigUint;

// Generates a one‑hot encoded vector in conventional (MSB‑first) order
// while keeping track of the underlying MSB-first "hot" index.
// In this representation, the printed string is as you usually see it (left: MSB, right: LSB),
// but the random index generated (lsb_hot_index) is directly usable for DPF key generation.
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

/// Generate Beaver triples for secure multiplication
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

/// Pre-processes and generates DPF keys for MAC computation
fn preprocess_mac(
    domain_size: usize,
    alpha_val: &FE,
) -> ((SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), (SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), FE, (FE, FE), (FE, FE)) {
    // Generate random position r as a usize
    let mut rng = rand::thread_rng();
    let r_usize = rng.gen_range(0, domain_size);
    println!("Random position r (usize): {}", r_usize);

    let nbits = (domain_size as f64).log2().ceil() as u8;

    // Convert r to FE
    let r: FE = FE::from(r_usize as u32); // Assuming FE implements From<u32>
    println!("Random position r (FE): {:?}", r);
    
    // Generate shares of r
    let (r0, r1) = generate_alpha_shares(&r);
    
    // Generate shares of alpha_val
    let (alpha0, alpha1) = generate_alpha_shares(alpha_val);
    
    let alpha = u32_to_bits(nbits, r_usize as u32);
    println!("Alpha: {:?}", alpha);
    let betas = vec![FE::one(); alpha.len() - 1];
    let beta_last = FE::one();
    let key_pair1 = SketchDPFKey::gen(&alpha, &betas, &beta_last);

    // Generate MAC DPF key pair (alpha at position r, 0 elsewhere)
    let betas2 = vec![FE::one(); alpha.len() - 1];
    let beta_last2 = alpha_val.clone();
    let key_pair2 = SketchDPFKey::gen(&alpha, &betas2, &beta_last2);

    (key_pair1.into(), key_pair2.into(), r, (r0, r1), (alpha0, alpha1))
}


/// Evaluates the SketchDPFKey for all values in the domain and returns the results as vectors.
fn eval_all(key: &SketchDPFKey<FE, FE>, domain_size: usize) -> Vec<FE>
{
    let mut all_values = Vec::with_capacity(domain_size);
    let nbits = (domain_size as f64).log2().ceil() as u8;

    for i in 0..domain_size {
        let bits = u32_to_bits(nbits, i as u32); // For a domain of 64 it is 6 bits
        let value = key.eval(&bits);
        all_values.push(value.clone());
    }

    all_values
}

// the PIKA based MAC checks
fn mal_preprocess_check(
    values1_0: &[FE], values1_1: &[FE],
    values2_0: &[FE], values2_1: &[FE],
    domain_size: usize,
    r: &FE,
    alpha_val: &FE,
    r0: &FE, r1: &FE,
    alpha_val_0: &FE, alpha_val_1: &FE) {
    
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
    
    // mac check (z_star) using alpha shares
    let alpha_val_recon = alpha_val_0.clone() + alpha_val_1.clone();
    let mac_check = alpha_val_recon * z1 - z_star;
    println!("MAC check: {:?}", mac_check.value());
}

fn main() {
    let num_clients = 100;
    let domain_size = 1280; // Domain size of 64 positions.

    let overall_start = Instant::now();

    // Part 1: Pre-processing
    println!("\n Pre-processing:");
    
    // a random alpha value for MAC
    let alpha_val = FE::random();
    
    // Pre-process and generate DPF keys, changed to get shares of r and alpha
    let ((key1_0, key1_1), (key2_0, key2_1), r, (r_0, r_1), (alpha_val_0, alpha_val_1)) = 
        preprocess_mac(domain_size, &alpha_val);
    
    // Evaluate all values for both key pairs, will use these to shift and lookup values
    let values1_0 = eval_all(&key1_0, domain_size);
    let values1_1 = eval_all(&key1_1, domain_size);
    let values2_0 = eval_all(&key2_0, domain_size);
    let values2_1 = eval_all(&key2_1, domain_size);

    // Now convert the column sum shares into their corresponding DPF values
    // do another round of generating a random r and shifting the values
    // IMP: the domain size here becomes equal to the number of clients + 1 actually (range for the sum to be)
    // this is because the column sum can be anywhere from 0 to num_clients which is (num_clients + 1) values
    let updated_domain = num_clients + 1;
    let ((key1_0, key1_1), (key2_0, key2_1), r2, (r2_0, r2_1), (alpha_val2_0, alpha_val2_1)) = 
        preprocess_mac(updated_domain, &alpha_val);
    // println!("r2 is: {}", r2);

    // Evaluate all values for both key pairs
    let col_sum_values1_0 = eval_all(&key1_0, updated_domain);
    let col_sum_values1_1 = eval_all(&key1_1, updated_domain);
    let col_sum_values2_0 = eval_all(&key2_0, updated_domain);
    let col_sum_values2_1 = eval_all(&key2_1, updated_domain);
    
    // Use the shares that were returned from preprocess_mac
    mal_preprocess_check(&col_sum_values1_0, &col_sum_values1_1, &col_sum_values2_0, &col_sum_values2_1, 
        updated_domain, &r2, &alpha_val, &r2_0, &r2_1, &alpha_val2_0, &alpha_val2_1);

    // Generate Beaver triples for MAC checks during preprocessing
    // TODO: check this
    let num_mac_checks_needed = num_clients + domain_size + 10;
    let beaver_triples = generate_beaver_triples(num_mac_checks_needed);
    
    mal_preprocess_check(&values1_0, &values1_1, &values2_0, &values2_1, domain_size, &r, &alpha_val, &r_0, &r_1, &alpha_val_0, &alpha_val_1);

    let preprocess_time = overall_start.elapsed();
    println!("Pre-processing took: {:?}", preprocess_time);


    // Part 2: Online computation
    let client_start = Instant::now();
    
    let mut all_client_s0 = vec![FE::zero(); domain_size];
    let mut all_client_s1 = vec![FE::zero(); domain_size];
    let mut all_client_m0 = vec![FE::zero(); domain_size];
    let mut all_client_m1 = vec![FE::zero(); domain_size];
    let mut x_val = vec![0; num_clients];

    // For each client, generate a different secret input a and run the lookup steps.
    for client in 0..num_clients {
        println!("\nClient {}:", client);

        // Generate client's one‑hot input.
        // client_input.0 is the one‑hot vector (MSB‑first) and client_input.1 is the LSB‑first hot index.
        let (one_hot, lsb_hot_index) = generate_one_hot_conventional(domain_size);
        // Convert to conventional index (0 = leftmost).
        let a_index = domain_size - 1 - lsb_hot_index;
        let a_val = FE::from(a_index as u32);
        println!("Secret input (a): {}", a_val);

        // Generate shares of client's input a
        let (a_0, a_1) = generate_alpha_shares(&a_val);

        // Use the shares of r that were already generated
        let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
        let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);

        let x_share0_fe = (r_0.clone() - a_0.clone()) % (FE::from(domain_size as u32));
        // open by summing shares:
        x_val[client] = (x_share0 + x_share1) % (domain_size as u64);

        // COMM ROUND for summing up x_val

        // Use a Beaver triple for secure MAC check by creating shares of [alpha*r]
        let triple_index = client;
        let (a0, a1, b0, b1, c0, c1) = &beaver_triples[triple_index];
        
        // After opening x_val[client]
        let x_opened = FE::from(x_val[client] as u32);
        
        // Using Beaver triples for MAC check
        // 1. Each party computes their share of d = alpha_val - a
        let d_0 = alpha_val_0.clone() - a0.clone();
        let d_1 = alpha_val_1.clone() - a1.clone();
        
        // 2. Parties open d = d_0 + d_1
        let d = d_0.clone() + d_1.clone();     
        
        // 3. Compute e = x_opened - b
        let b_combined = b0.clone() + b1.clone();
        let e = x_opened.clone() - b_combined;

        let de = d.clone() * e.clone();

        // 4. Each party computes their share of MAC = alpha*x (FIX #2: Add d*e term to Party 0's share)
        let mut mac_x_0 = d.clone() * b0.clone() + e.clone() * a0.clone() + c0.clone() + de.clone();
        let mut mac_x_1 = d.clone() * b1.clone() + e.clone() * a1.clone() + c1.clone();
        
        // Now use these MAC shares for MAC check
        // Party 0 computes their share of z = x*alpha_0 - mac_x_0
        let mut z_0 = x_opened.clone();
        z_0.mul(&alpha_val_0);
        z_0.sub(&mac_x_0);
        
        // Party 1 computes their share of z
        let mut z_1 = x_opened.clone();
        z_1.mul(&alpha_val_1);
        z_1.sub(&mac_x_1);
        
        // Open z by combining shares
        let z_opened = z_0 + z_1;
        
        // Check MAC
        if z_opened.value() != 0 {
            panic!("MAC failure on r-a opening for client {}", client);
        }

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

        // Compute cumulative sum of shifted_values so that we can get shares of the 111000 form and the MACs
        // IMP : We're directly adding to get the column wise sums of shares for all clients in all_clients_s0 
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
    // do another round of using a random r and shifting the values
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

        // Use another Beaver triple for MAC check of x2_val
        let triple_index = num_clients + idx as usize;
        let (a0, a1, b0, b1, c0, c1) = &beaver_triples[triple_index];
        
        // After opening x2_val
        let x2_opened = FE::from(x2_val as u32);
        
        // Using Beaver triples for MAC check
        // 1. Each party computes their share of d = alpha_val2 - a
        let d_0 = alpha_val2_0.clone() - a0.clone();
        let d_1 = alpha_val2_1.clone() - a1.clone();
        
        // 2. Parties open d = d_0 + d_1
        let d = d_0.clone() + d_1.clone();
        
        // 3. Compute e = x2_opened - b
        let e = x2_opened.clone() - b0.clone() - b1.clone();

        let de = d.clone() * e.clone();
        
        // 4. Each party computes their share of MAC = alpha*x2
        let mut mac_x2_0 = d.clone() * b0.clone() + e.clone() * a0.clone() + c0.clone() + de.clone();
        let mut mac_x2_1 = d.clone() * b1.clone() + e.clone() * a1.clone() + c1.clone();
        
        // Party 0 computes their share of z = x2*alpha_2_0 - mac_x2_0
        let mut z_0 = x2_opened.clone();
        z_0.mul(&alpha_val2_0);
        z_0.sub(&mac_x2_0);
        
        // Party 1 computes their share of z
        let mut z_1 = x2_opened.clone();
        z_1.mul(&alpha_val2_1);
        z_1.sub(&mac_x2_1);
        
        // Open z by combining shares
        let z_opened = z_0 + z_1;
        
        // Check MAC
        if z_opened.value() != 0 {
            panic!("MAC failure on r2-col_sum opening for idx {}", idx);
        }

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
        let mut col_sum_s = FE::zero();
        let mut col_sum_m = FE::zero();
        let n1 = updated_domain - 1;
        let n2 = updated_domain - 2;
        for &j in &[n2, n1] {
            col_sum_s.add(&col_sum_shifted_val_1_0[j]);
            col_sum_s.add(&col_sum_shifted_val_1_1[j]);
            col_sum_m.add(&col_sum_shifted_val_2_0[j]);
            col_sum_m.add(&col_sum_shifted_val_2_1[j]);
        }
        
        if col_sum_s.value() >= 1 { 
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
                for index in 0..=idx { // TODO: optimize to use the shifted values from before instead of re-computing more
                    let sh_index = (index + x_val[bidder] as usize) % domain_size;
                    temp_sum.add(&values1_0[sh_index].clone());
                    temp_sum.add(&values1_1[sh_index].clone());
                    temp_sum_mac.add(&values2_0[sh_index].clone());
                    temp_sum_mac.add(&values2_1[sh_index].clone());
                }

                // **MAC check on highest-bidder reveal**
                let mut expected_mac = temp_sum.clone();
                expected_mac.mul(&alpha_val);
                if temp_sum_mac.value() != expected_mac.value() {
                    panic!("MAC failure on highest-bidder reveal for bidder {}", bidder);
                }
                let high_bid_idx: u64 = temp_sum.value();
                if high_bid_idx == 0 {
                    println!(" The index of highest bidder is: {}", bidder);
                }
            }
            break;
        }
    }

    let client_duration = client_start.elapsed();
    println!("Pre-processing took: {:?}", preprocess_time);
    println!("Online time: {:?}", client_duration);
}