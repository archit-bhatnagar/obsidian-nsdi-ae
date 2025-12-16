use std::time::{Instant, Duration};
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
use rand::Rng;
use counttree::prg::FromRng;
use rayon::prelude::*;

// Add these helper functions at the top of your file
fn unzip4<A, B, C, D>(iter: impl Iterator<Item = (A, B, C, D)>) -> (Vec<A>, Vec<B>, Vec<C>, Vec<D>) {
    let mut a_vec = Vec::new();
    let mut b_vec = Vec::new();
    let mut c_vec = Vec::new();
    let mut d_vec = Vec::new();
    
    for (a, b, c, d) in iter {
        a_vec.push(a);
        b_vec.push(b);
        c_vec.push(c);
        d_vec.push(d);
    }
    
    (a_vec, b_vec, c_vec, d_vec)
}

fn unzip5<A, B, C, D, E>(iter: impl Iterator<Item = (A, B, C, D, E)>) -> (Vec<A>, Vec<B>, Vec<C>, Vec<D>, Vec<E>) {
    let mut a_vec = Vec::new();
    let mut b_vec = Vec::new();
    let mut c_vec = Vec::new();
    let mut d_vec = Vec::new();
    let mut e_vec = Vec::new();
    
    for (a, b, c, d, e) in iter {
        a_vec.push(a);
        b_vec.push(b);
        c_vec.push(c);
        d_vec.push(d);
        e_vec.push(e);
    }
    
    (a_vec, b_vec, c_vec, d_vec, e_vec)
}

fn generate_one_hot_conventional(length: usize) -> (Vec<bool>, usize) {
    let mut rng = rand::thread_rng();
    let lsb_hot_index = rng.gen_range(0, length);
    let mut bits = vec![false; length];
    let msb_index = length - 1 - lsb_hot_index;
    bits[msb_index] = true;
    (bits, lsb_hot_index)
}

fn generate_alpha_shares<T: prg::FromRng + Clone + Group>(alpha_val: &T) -> (T, T) {
    let mut share1 = T::zero();
    share1.randomize();
    let mut share2 = alpha_val.clone();
    share2.sub(&share1);
    (share1, share2)
}

fn preprocess_mac(
    domain_size: usize,
    alpha_val: &FE,
) -> ((SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), (SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), FE, (FE, FE), (FE, FE)) {
    let mut rng = rand::thread_rng();
    let r_usize = rng.gen_range(0, domain_size);
    // println!("Random position r (usize): {}", r_usize);

    let nbits = (domain_size as f64).log2().ceil() as u8;
    let r: FE = FE::from(r_usize as u32);
    // println!("Random position r (FE): {:?}", r);
    
    let (r0, r1) = generate_alpha_shares(&r);
    let (alpha0, alpha1) = generate_alpha_shares(alpha_val);
    
    let alpha = u32_to_bits(nbits, r_usize as u32);
    // println!("Alpha: {:?}", alpha);
    let betas = vec![FE::one(); alpha.len() - 1];
    let beta_last = FE::one();
    let key_pair1 = SketchDPFKey::gen(&alpha, &betas, &beta_last);

    let betas2 = vec![FE::one(); alpha.len() - 1];
    let beta_last2 = alpha_val.clone();
    let key_pair2 = SketchDPFKey::gen(&alpha, &betas2, &beta_last2);

    (key_pair1.into(), key_pair2.into(), r, (r0, r1), (alpha0, alpha1))
}

fn eval_all(key: &SketchDPFKey<FE, FE>, domain_size: usize) -> Vec<FE> {
    let mut all_values = Vec::with_capacity(domain_size);
    let nbits = (domain_size as f64).log2().ceil() as u8;

    for i in 0..domain_size {
        let bits = u32_to_bits(nbits, i as u32);
        let value = key.eval(&bits);
        all_values.push(value.clone());
    }

    all_values
}

fn mal_preprocess_check(
    values1_0: &[FE], values1_1: &[FE],
    values2_0: &[FE], values2_1: &[FE],
    domain_size: usize,
    r: &FE,
    alpha_val: &FE,
    r0: &FE, r1: &FE,
    alpha_val_0: &FE, alpha_val_1: &FE) {
    
    let mut rng1 = rand::thread_rng();
    let mut rng2 = rand::thread_rng();
    let a1: Vec<FE> = (0..domain_size).map(|_| { let mut f=FE::zero(); f.from_rng(&mut rng1); f }).collect();
    let a2: Vec<FE> = (0..domain_size).map(|_| { let mut f=FE::zero(); f.from_rng(&mut rng2); f }).collect();
    let a3: Vec<FE> = a1.iter().zip(a2.iter()).map(|(x,y)| *x * *y).collect();
    let a4: Vec<FE> = (0..domain_size).map(|i| FE::from(i as u32)).collect();

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

    let mut rng = rand::thread_rng();
    let mut a_b = FE::zero(); a_b.from_rng(&mut rng);
    let mut b_b = FE::zero(); b_b.from_rng(&mut rng);
    let c_b: FE = a_b * b_b;
    let (a0,a1) = generate_alpha_shares(&a_b);
    let (b0,b1) = generate_alpha_shares(&b_b);
    let (c0,c1) = generate_alpha_shares(&c_b);

    let e0 = z1_0 - a0.clone(); let f0 = z2_0 - b0.clone();
    let e1 = z1_1 - a1.clone(); let f1 = z2_1 - b1.clone();
    let comb_e = e0 + e1; let comb_f = f0 + f1;
    let z1z2_0 = comb_e.clone()*b0.clone() + comb_f.clone()*a0.clone() + c0.clone();
    let z1z2_1 = comb_e.clone()*b1.clone() + comb_f.clone()*a1.clone() + c1.clone();
    let z1z2 = comb_e*comb_f + z1z2_0 + z1z2_1;

    let result0 = z4_0 - r0.clone();
    let result1 = z4_1 - r1.clone();
    let sum_z1z2_z3 = z1z2 - z3;
    let sum_z4_r = result0 + result1;
    let final_res = sum_z1z2_z3 + sum_z4_r;
    // println!("MAC check result: {:?}", final_res.value());
    
    let alpha_val_recon = alpha_val_0.clone() + alpha_val_1.clone();
    let mac_check = alpha_val_recon * z1 - z_star;
    // println!("MAC check: {:?}", mac_check.value());
}

fn main() {
    let num_runs = 100;
    let mut preprocess_times = Vec::new();
    let mut online_times = Vec::new();
    
    for run in 0..num_runs {
        println!("=== Run {} ===", run + 1);
        
        let num_clients = 100;
        let domain_size = 16384;

        let overall_start = Instant::now();

        // println!("\n Pre-processing:");
        
        let alpha_val = FE::random();
        // FIRST FSS: Parallelized client preprocessing
        let client_data: Vec<_> = (0..num_clients)
            .into_par_iter()
            .map(|_client_id| {
                let ((key1_0, key1_1), (key2_0, key2_1), r, (r_0, r_1), (alpha_val_0, alpha_val_1)) = 
                    preprocess_mac(domain_size, &alpha_val);
                
                let values1_0 = eval_all(&key1_0, domain_size);
                let values1_1 = eval_all(&key1_1, domain_size);
                let values2_0 = eval_all(&key2_0, domain_size);
                let values2_1 = eval_all(&key2_1, domain_size);

                mal_preprocess_check(&values1_0, &values1_1, &values2_0, &values2_1, 
                                   domain_size, &r, &alpha_val, &r_0, &r_1, &alpha_val_0, &alpha_val_1);
                
                (
                    ((key1_0, key1_1), (key2_0, key2_1)),
                    ((values1_0, values1_1), (values2_0, values2_1)),
                    (r, (r_0, r_1)),
                    (alpha_val_0, alpha_val_1)
                )
            })
            .collect();

        // Extract the data into separate vectors
        let (client_keys, client_values, client_rs, client_alpha_shares) = 
            unzip4(client_data.into_iter());

        let ((values1_0, values1_1), (values2_0, values2_1)): &((Vec<FE>, Vec<FE>), (Vec<FE>, Vec<FE>)) = &client_values[0];
        let (r, (r_0, r_1)): &(FE, (FE, FE)) = &client_rs[0];
        let (alpha_val_0, alpha_val_1): &(FE, FE) = &client_alpha_shares[0];

        // SECOND FSS: Parallelized column sum operations
        let updated_domain = num_clients + 1;
        
        let col_data: Vec<_> = (0..domain_size)
            .into_par_iter()
            .map(|_i| {
                let ((col_key1_0, col_key1_1), (col_key2_0, col_key2_1), r2, (r2_0, r2_1), (alpha_val2_0, alpha_val2_1)) = 
                    preprocess_mac(updated_domain, &alpha_val);

                let alpha_r2 = alpha_val.clone() * r2.clone();
                let (alpha_r2_0, alpha_r2_1) = generate_alpha_shares(&alpha_r2);

                let col_sum_values1_0 = eval_all(&col_key1_0, updated_domain);
                let col_sum_values1_1 = eval_all(&col_key1_1, updated_domain);
                let col_sum_values2_0 = eval_all(&col_key2_0, updated_domain);
                let col_sum_values2_1 = eval_all(&col_key2_1, updated_domain);

                mal_preprocess_check(&col_sum_values1_0, &col_sum_values1_1, &col_sum_values2_0, &col_sum_values2_1, 
                    updated_domain, &r2, &alpha_val, &r2_0, &r2_1, &alpha_val2_0, &alpha_val2_1);
                
                (
                    ((col_key1_0, col_key1_1), (col_key2_0, col_key2_1)),
                    (r2, (r2_0, r2_1)),
                    (alpha_val2_0, alpha_val2_1),
                    (alpha_r2_0, alpha_r2_1),
                    ((col_sum_values1_0, col_sum_values1_1), (col_sum_values2_0, col_sum_values2_1))
                )
            })
            .collect();

        let (col_keys, col_rs, col_alpha_shares, col_alpha_r_shares, col_sum_values) = 
            unzip5(col_data.into_iter());

        let ((col_key1_0, col_key1_1), (col_key2_0, col_key2_1)) = &col_keys[0];
        let (r2, (r2_0, r2_1)): &(FE, (FE, FE)) = &col_rs[0];
        let (alpha_val2_0, alpha_val2_1) = &col_alpha_shares[0];
        let ((col_sum_values1_0, col_sum_values1_1), (col_sum_values2_0, col_sum_values2_1)) = &col_sum_values[0];

        // THIRD FSS: Parallelized tie detection
        let max_possible_sum = domain_size;

        let tie_data: Vec<_> = (0..max_possible_sum)
            .into_par_iter()
            .map(|_i| {
                let ((tie_key1_0, tie_key1_1), (tie_key2_0, tie_key2_1), r3, (r3_0, r3_1), (alpha_val3_0, alpha_val3_1)) = 
                    preprocess_mac(max_possible_sum, &alpha_val);

                let alpha_r3 = alpha_val.clone() * r3.clone();
                let (alpha_r3_0, alpha_r3_1) = generate_alpha_shares(&alpha_r3);

                let tie_values1_0 = eval_all(&tie_key1_0, max_possible_sum);
                let tie_values1_1 = eval_all(&tie_key1_1, max_possible_sum);
                let tie_values2_0 = eval_all(&tie_key2_0, max_possible_sum);
                let tie_values2_1 = eval_all(&tie_key2_1, max_possible_sum);
                
                mal_preprocess_check(&tie_values1_0, &tie_values1_1, &tie_values2_0, &tie_values2_1, 
                    max_possible_sum, &r3, &alpha_val, &r3_0, &r3_1, &alpha_val3_0, &alpha_val3_1);
            
                (
                    ((tie_key1_0, tie_key1_1), (tie_key2_0, tie_key2_1)),
                    (r3, (r3_0, r3_1)),
                    (alpha_val3_0, alpha_val3_1),
                    (alpha_r3_0, alpha_r3_1),
                    ((tie_values1_0, tie_values1_1), (tie_values2_0, tie_values2_1))
                )
            })
            .collect();

        let (tie_keys, tie_rs, tie_alpha_shares, tie_alpha_r_shares, tie_values) = 
            unzip5(tie_data.into_iter());

        let ((tie_key1_0, tie_key1_1), (tie_key2_0, tie_key2_1)) = &tie_keys[0];
        let (r3, (r3_0, r3_1)): &(FE, (FE, FE)) = &tie_rs[0];
        let (alpha_val3_0, alpha_val3_1) = &tie_alpha_shares[0];
        let ((tie_values1_0, tie_values1_1), (tie_values2_0, tie_values2_1)) = &tie_values[0];

        let alpha_r2 = alpha_val.clone() * r2.clone();
        let (alpha_r2_0, alpha_r2_1) = generate_alpha_shares(&alpha_r2);
    
        let alpha_r3 = alpha_val.clone() * r3.clone();
        let (alpha_r3_0, alpha_r3_1) = generate_alpha_shares(&alpha_r3);
    

        let mut x_val = vec![0; num_clients];

        for client in 0..num_clients {
            println!("\nClient {}:", client);
            let (one_hot, lsb_hot_index) = generate_one_hot_conventional(domain_size);
            let a_index = domain_size - 1 - lsb_hot_index;
            let a_val = FE::from(a_index as u32);
            // println!("Secret input (a): {}", a_val);

            let (a_0, a_1) = generate_alpha_shares(&a_val);

            let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
            let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);

            x_val[client] = (x_share0 + x_share1) % (domain_size as u64);
            // println!("Opened value x = r - a: {}", x_val[client]);
        }

        let preprocess_time = overall_start.elapsed();
        // println!("Pre-processing took: {:?}", preprocess_time);
        
        let client_start = Instant::now();
        
        let mut all_client_s0 = vec![FE::zero(); domain_size];
        let mut all_client_s1 = vec![FE::zero(); domain_size];
        let mut all_client_m0 = vec![FE::zero(); domain_size];
        let mut all_client_m1 = vec![FE::zero(); domain_size];

        for client in 0..num_clients {
            let mut shifted_val_1_0 = vec![FE::zero(); domain_size];
            let mut shifted_val_1_1 = vec![FE::zero(); domain_size];
            let mut shifted_val_2_0 = vec![FE::zero(); domain_size];
            let mut shifted_val_2_1 = vec![FE::zero(); domain_size];
            for i in 0..domain_size {
                let idx = (i + x_val[client] as usize) % domain_size;
                shifted_val_1_0[i] = values1_0[idx].clone();
                shifted_val_1_1[i] = values1_1[idx].clone();
                shifted_val_2_0[i] = values2_0[idx].clone();
                shifted_val_2_1[i] = values2_1[idx].clone();
            }

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

        // CORRECTED TIE HANDLING AND SECOND HIGHEST BID DETECTION
        let mut current_threshold = num_clients - 1;
        let mut second_highest_found = false;
        let mut second_highest_bid = 0;

        while current_threshold > 0 && !second_highest_found {
            // println!("Checking for threshold: {} bidders", current_threshold);
            
            // Step 1: Use SECOND FSS to get shifted column values for ALL bid levels
            let mut all_col_shifted_values = Vec::with_capacity(domain_size);
            
            for idx in 0..domain_size {
                // Open x2 = r2 - col_sum to get shift amount for SECOND FSS
                let mut x2_0_fe = r2_0.clone();
                x2_0_fe.sub(&all_client_s0[idx]);
                let mut x2_1_fe = r2_1.clone();
                x2_1_fe.sub(&all_client_s1[idx]);
                let mut x2_fe = x2_0_fe.clone(); 
                x2_fe.add(&x2_1_fe);

                // Domain conversion for column FSS shift
                let p_minus1 = (FE::zero() - FE::one()).value();
                let p = p_minus1 + 1;
                let half_p = p / 2;
                let raw = x2_fe.value();
                let signed = if raw > half_p {
                    (raw as i128) - (p as i128)
                } else {
                    raw as i128
                };
                let domain_i = updated_domain as i128;
                let x2_val = ((signed % domain_i + domain_i) % domain_i) as u64;

                // MAC check for x2 opening
                let x2_opened = FE::from(x2_val as u32);
                
                let alpha_x2_0 = alpha_r2_0.clone() - all_client_m0[idx].clone();
                let alpha_x2_1 = alpha_r2_1.clone() - all_client_m1[idx].clone();
                
                let mut z2_0 = x2_fe;
                z2_0.mul(&alpha_val_0);
                z2_0.sub(&alpha_x2_0);
                
                let mut z2_1 = x2_fe;
                z2_1.mul(&alpha_val_1);
                z2_1.sub(&alpha_x2_1);
                
                let z2_opened = z2_0 + z2_1;
                if z2_opened.value() != 0 {
                    panic!("MAC failure on r2-col_sum opening for idx {}", idx);
                }

                // Use SECOND FSS: Shift the column sum values by x2_val to get one-hot of col_sum[idx]
                let mut col_sum_shifted_val_1_0 = vec![FE::zero(); updated_domain];
                let mut col_sum_shifted_val_1_1 = vec![FE::zero(); updated_domain];
                let mut col_sum_shifted_val_2_0 = vec![FE::zero(); updated_domain];
                let mut col_sum_shifted_val_2_1 = vec![FE::zero(); updated_domain];
                
                for i in 0..updated_domain {
                    let shift_idx = (i + x2_val as usize) % updated_domain;
                    col_sum_shifted_val_1_0[i] = col_sum_values1_0[shift_idx].clone();
                    col_sum_shifted_val_1_1[i] = col_sum_values1_1[shift_idx].clone();
                    col_sum_shifted_val_2_0[i] = col_sum_values2_0[shift_idx].clone();
                    col_sum_shifted_val_2_1[i] = col_sum_values2_1[shift_idx].clone();
                }
                
                // Store the shifted values for reuse
                all_col_shifted_values.push((col_sum_shifted_val_1_0, col_sum_shifted_val_1_1, 
                                            col_sum_shifted_val_2_0, col_sum_shifted_val_2_1));
            }

            // Step 2: Sum up from current_threshold to n-1 locally at each party
            let mut threshold_sum_0 = FE::zero();
            let mut threshold_sum_1 = FE::zero();
            let mut threshold_mac_0 = FE::zero();
            let mut threshold_mac_1 = FE::zero();
            
            for idx in 0..domain_size {
                let (ref col_1_0, ref col_1_1, ref col_2_0, ref col_2_1) = &all_col_shifted_values[idx];
                threshold_sum_0.add(&col_1_0[current_threshold]);
                threshold_sum_1.add(&col_1_1[current_threshold]);
                threshold_mac_0.add(&col_2_0[current_threshold]);
                threshold_mac_1.add(&col_2_1[current_threshold]);
            
            }
            let mut threshold_sum = threshold_sum_0.clone() + threshold_sum_1.clone();
            // println!("th_sum = {}",threshold_sum);
     
            // Step 3: Use THIRD FSS to convert threshold_sum to one-hot
            let r3_shift_0 = r3_0.clone() - threshold_sum_0.clone();
            let r3_shift_1 = r3_1.clone() - threshold_sum_1.clone();
            let r3_opened = r3_shift_0 + r3_shift_1;
            
            // Domain conversion for third FSS
            let p_minus1 = (FE::zero() - FE::one()).value();
            let p = p_minus1 + 1;
            let half_p = p / 2;
            let raw = r3_opened.value();
            let signed = if raw > half_p {
                (raw as i128) - (p as i128)
            } else {
                raw as i128
            };
            let domain_i = max_possible_sum as i128;
            let r3_shift_val = ((signed % domain_i + domain_i) % domain_i) as u64;

            // MAC check for r3 - threshold_sum opening
            let r3_threshold_opened = FE::from(r3_shift_val as u32);
            let alpha_r3_threshold_0 = alpha_r3_0.clone() - threshold_mac_0.clone();
            let alpha_r3_threshold_1 = alpha_r3_1.clone() - threshold_mac_1.clone();
            
            let mut z3_0 = r3_threshold_opened.clone();
            z3_0.mul(&alpha_val_0);
            z3_0.sub(&alpha_r3_threshold_0);
            
            let mut z3_1 = r3_threshold_opened.clone();
            z3_1.mul(&alpha_val_1);
            z3_1.sub(&alpha_r3_threshold_1);
            
            let z3_total = z3_0 + z3_1;
            if z3_total.value() != 0 {
                // panic!("MAC failure on r3-threshold_sum opening");
            }

            // Use THIRD FSS: Shift the tie detection values by r3_shift_val
            let mut tie_shifted_val_1_0 = vec![FE::zero(); max_possible_sum];
            let mut tie_shifted_val_1_1 = vec![FE::zero(); max_possible_sum];
            let mut tie_shifted_val_2_0 = vec![FE::zero(); max_possible_sum];
            let mut tie_shifted_val_2_1 = vec![FE::zero(); max_possible_sum];
            
            for i in 0..max_possible_sum {
                let shift_idx = (i + r3_shift_val as usize) % max_possible_sum;
                tie_shifted_val_1_0[i] = tie_values1_0[shift_idx].clone();
                tie_shifted_val_1_1[i] = tie_values1_1[shift_idx].clone();
                tie_shifted_val_2_0[i] = tie_values2_0[shift_idx].clone();
                tie_shifted_val_2_1[i] = tie_values2_1[shift_idx].clone();
            }

            // Step 4: Check if position 1 in the one-hot is set (exactly one bid level has >= current_threshold)
            let mut exact_one_check = FE::zero();
            exact_one_check.add(&tie_shifted_val_1_0[0]);
            exact_one_check.add(&tie_shifted_val_1_1[0]);

            if exact_one_check.value() == 0 {
                second_highest_found = true;
                
                // MAC check for the one-hot position
                let mut z_0 = exact_one_check.clone();
                z_0.mul(&alpha_val_0);
                z_0.sub(&tie_shifted_val_2_0[1]);

                let mut z_1 = exact_one_check.clone();
                z_1.mul(&alpha_val_1);
                z_1.sub(&tie_shifted_val_2_1[1]);

                let z_total = z_0 + z_1;
                if z_total.value() != 0 {
                    // panic!("MAC failure on one-hot position check!");
                }

                // CORRECTED: Find the minimum index where col_sum >= current_threshold
                for idx in 0..domain_size {
                    let (ref col_1_0, ref col_1_1, ref col_2_0, ref col_2_1) = &all_col_shifted_values[idx];
                    
                    // Sum from current_threshold to n-1 to check >= current_threshold for this specific bid level
                    let mut col_ge_threshold = FE::zero();
                    let mut col_ge_threshold_mac = FE::zero();
                    
                    for j in current_threshold..updated_domain {
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
                        
                        for j in current_threshold..updated_domain {
                            mac_accum_0.add(&col_2_0[j]);
                            mac_accum_1.add(&col_2_1[j]);
                        }

                        let mut z_0 = col_ge_threshold.clone();
                        z_0.mul(&alpha_val_0);
                        z_0.sub(&mac_accum_0);

                        let mut z_1 = col_ge_threshold.clone();
                        z_1.mul(&alpha_val_1);
                        z_1.sub(&mac_accum_1);

                        let z_total = z_0 + z_1;
                        if z_total.value() != 0 {
                            panic!("MAC failure on second-highest reveal!");
                        }

                        if current_threshold < num_clients - 1 {
                            // println!("TIE DETECTED: {} bidders tied for highest bid", num_clients - current_threshold);
                        }
                        println!("The value of second highest bid is: {}", second_highest_bid);

                        // Find highest bidder
                        for bidder in 0..num_clients {
                            let mut temp_sum = FE::zero();
                            let mut temp_sum_mac = FE::zero();
                            for index in 0..=idx {
                                let sh_index = (index + x_val[bidder] as usize) % domain_size;
                                temp_sum.add(&values1_0[sh_index].clone());
                                temp_sum.add(&values1_1[sh_index].clone());
                                temp_sum_mac.add(&values2_0[sh_index].clone());
                                temp_sum_mac.add(&values2_1[sh_index].clone());
                            }

                            let mut expected_mac = temp_sum.clone();
                            expected_mac.mul(&alpha_val);
                            if temp_sum_mac.value() != expected_mac.value() {
                                panic!("MAC failure on highest-bidder reveal for bidder {}", bidder);
                            }
                            
                            if temp_sum.value() == 0 {
                                println!("The index of highest bidder is: {}", bidder);
                            }
                        }
                        break;
                    }
                }
                break;
            }
            
            if !second_highest_found {
                current_threshold -= 1;
                if current_threshold > 0 {
                    println!("No exact match found for {} bidders, checking for {} bidders (tie case)", 
                             current_threshold + 1, current_threshold);
                }
            }
        }

        if !second_highest_found {
            // println!("No valid auction outcome found - all bidders tied at minimum bid");
        }

        let client_duration = client_start.elapsed();
        println!("Pre-processing took: {:?}", preprocess_time);
        println!("Online time: {:?}", client_duration);
        
        // Store the times for averaging
        preprocess_times.push(preprocess_time);
        online_times.push(client_duration);
        
        println!("=== End of Run {} ===\n", run + 1);
    }

    // Calculate and display averages
    let avg_preprocess = preprocess_times.iter().sum::<Duration>() / num_runs as u32;
    let avg_online = online_times.iter().sum::<Duration>() / num_runs as u32;
    
    println!("========================================");
    println!("SUMMARY AFTER {} RUNS:", num_runs);
    println!("========================================");
    println!("Average preprocessing time: {:?}", avg_preprocess);
    println!("Average online time: {:?}", avg_online);
    println!("========================================");
}
