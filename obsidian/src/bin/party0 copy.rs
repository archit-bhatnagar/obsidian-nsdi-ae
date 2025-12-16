use std::net::TcpListener;
use std::net::TcpStream;
// use tokio::net::TcpStream;
use std::time::Instant;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
use rand::Rng;
use counttree::prg::FromRng;
mod common;
use common::{Message, send_message, receive_message, fe_to_bytes, bytes_to_fe, generate_alpha_shares};

fn main() {
    println!("Party 0 (Server) starting...");
    let num_clients = 20;
    let domain_size = 32;

    let listener = TcpListener::bind("127.0.0.1:8888").expect("Failed to bind to address");
    println!("Listening on port 8888");
    let (mut stream, _) = listener.accept().expect("Failed to accept connection");
    println!("Party 1 connected");

    let overall_start = Instant::now();

    // === Preprocessing Phase ===
    let alpha_val = FE::random();
    let ((key1_0, key1_1), (key2_0, key2_1), r, (r_0, r_1), (alpha_val_0, alpha_val_1)) =
        preprocess_mac(domain_size, &alpha_val);

    send_message(&mut stream, &Message::AlphaShare(fe_to_bytes(&alpha_val_1))).unwrap();
    send_message(&mut stream, &Message::RShare(fe_to_bytes(&r_1))).unwrap();

    let values1_0 = eval_all(&key1_0, domain_size);
    let values1_1 = eval_all(&key1_1, domain_size);
    let values2_0 = eval_all(&key2_0, domain_size);
    let values2_1 = eval_all(&key2_1, domain_size);

    // After sending Beaver triples:
    let values1_1_bytes: Vec<Vec<u8>> = values1_1.iter().map(|fe| fe_to_bytes(fe)).collect();
    let values2_1_bytes: Vec<Vec<u8>> = values2_1.iter().map(|fe| fe_to_bytes(fe)).collect();

    send_message(&mut stream, &Message::DPFValueShares(values1_1_bytes)).unwrap();
    send_message(&mut stream, &Message::DPFValueShares(values2_1_bytes)).unwrap();


    let num_mac_checks_needed = num_clients * 2 + domain_size + 10;
    let beaver_triples = generate_beaver_triples(num_mac_checks_needed);
    let mut triple_shares = Vec::with_capacity(num_mac_checks_needed);
    for (_, a1, _, b1, _, c1) in &beaver_triples {
        triple_shares.push((fe_to_bytes(a1), fe_to_bytes(b1), fe_to_bytes(c1)));
    }
    send_message(&mut stream, &Message::BeaverTripleShares(triple_shares)).unwrap();

    mal_preprocess_check(
        &values1_0, &values1_1, &values2_0, &values2_1,
        domain_size, &r, &alpha_val, &r_0, &r_1, &alpha_val_0, &alpha_val_1);

    let updated_domain = num_clients + 1;
    let ((key1_0, key1_1), (key2_0, key2_1), r2, (r2_0, r2_1), (alpha_val2_0, alpha_val2_1)) =
        preprocess_mac(updated_domain, &alpha_val);

    // FIXED THIS    
    send_message(&mut stream, &Message::AlphaShare(fe_to_bytes(&alpha_val2_1))).unwrap();
    send_message(&mut stream, &Message::R2Share(fe_to_bytes(&r2_1))).unwrap();

    let col_values1_0 = eval_all(&key1_0, updated_domain);
    let col_values1_1 = eval_all(&key1_1, updated_domain);
    let col_values2_0 = eval_all(&key2_0, updated_domain);
    let col_values2_1 = eval_all(&key2_1, updated_domain);

    let col_values1_1_bytes: Vec<Vec<u8>> = col_values1_1.iter().map(|fe| fe_to_bytes(fe)).collect();
    let col_values2_1_bytes: Vec<Vec<u8>> = col_values2_1.iter().map(|fe| fe_to_bytes(fe)).collect();

    send_message(&mut stream, &Message::DPFValueShares(col_values1_1_bytes)).unwrap();
    send_message(&mut stream, &Message::DPFValueShares(col_values2_1_bytes)).unwrap();

    mal_preprocess_check(
        &col_values1_0, &col_values1_1, &col_values2_0, &col_values2_1,
        updated_domain, &r2, &alpha_val, &r2_0, &r2_1, &alpha_val2_0, &alpha_val2_1);

    let preprocess_time = overall_start.elapsed();
    println!("Pre-processing took: {:?}", preprocess_time);

    // ready message
    let _: Message = receive_message(&mut stream).unwrap();

    let client_start = Instant::now();

    let mut all_client_s0 = vec![FE::zero(); domain_size];
    let mut all_client_m0 = vec![FE::zero(); domain_size];
    let mut x_val = vec![0; num_clients];

    for client in 0..num_clients {
        println!("\nClient {}:", client);
        let Message::ClientInput(a1_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected client input");
        };
        let a_1 = bytes_to_fe(&a1_bytes);

        let a_index = rand::thread_rng().gen_range(0, domain_size);
        let a_val = FE::from(a_index as u32);
        let (a_0, _) = generate_alpha_shares(&a_val);

        // let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
        let x_share0 = r_0 - a_0;
        send_message(&mut stream, &Message::X2Share(fe_to_bytes(&x_share0))).unwrap();
        let Message::X2Share(x_share1_vec) = receive_message(&mut stream).unwrap() else {
            panic!("Expected XShare");
        };
        let x_share1 = bytes_to_fe(&x_share1_vec);

        // open by summing shares:
        let x_opened = (x_share0 + x_share1);
        // let x_opened = FE::from(x_val[client] as u32);

        println!("x check is {}", x_opened.value());

        let (a0, a1, b0, b1, c0, c1) = &beaver_triples[client];
        let delta_0 = alpha_val_0.clone() - a0.clone();
        send_message(&mut stream, &Message::DeltaShare(fe_to_bytes(&delta_0))).unwrap();
        let Message::DeltaShare(delta_1_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected DeltaShare");
        };
        let delta_1 = bytes_to_fe(&delta_1_bytes);
        let delta = delta_0.clone() + delta_1.clone();

        let epsilon_0 = x_share0 - b0.clone();
        send_message(&mut stream, &Message::EpsilonShare(fe_to_bytes(&epsilon_0))).unwrap();
        let Message::EpsilonShare(epsilon_1_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected EpsilonShare");
        };
        let epsilon_1 = bytes_to_fe(&epsilon_1_bytes);
        let epsilon = epsilon_0.clone() + epsilon_1.clone();

        let de = delta.clone() * epsilon.clone();

        let mut mac_x_0 = c0.clone();
        mac_x_0.add(&(epsilon.clone() * a0.clone()));
        mac_x_0.add(&(delta.clone() * b0.clone()));
        mac_x_0.add(&de);

        let mut z_0 = x_opened.clone();
        z_0.mul(&alpha_val_0);
        z_0.sub(&mac_x_0);

        send_message(&mut stream, &Message::ZShare(fe_to_bytes(&z_0))).unwrap();
        let Message::ZShare(z_1_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected ZShare");
        };
        let z_1 = bytes_to_fe(&z_1_bytes);
        let z = z_0.clone() + z_1.clone();
        if z.value() != 0 {
            panic!("MAC failure on r-a opening for client {}", client);
        }
        
        println!("Opened value x = r - a (without domain): {}", x_opened.value());

        let mut shifted_val_1_0 = vec![FE::zero(); domain_size];
        let mut shifted_val_2_0 = vec![FE::zero(); domain_size];
        for i in 0..domain_size {
            let idx = (i + x_opened.value() as usize) % domain_size;
            shifted_val_1_0[i] = values1_0[idx].clone();
            shifted_val_2_0[i] = values2_0[idx].clone();
        }
        let mut cumulative_s0 = FE::zero();
        let mut cumulative_m0 = FE::zero();
        for i in 0..domain_size {
            cumulative_s0.add(&shifted_val_1_0[i]);
            all_client_s0[i].add(&cumulative_s0.clone());
            cumulative_m0.add(&shifted_val_2_0[i]);
            all_client_m0[i].add(&cumulative_m0.clone());
        }
    }

    // === Column Phase ===
    println!("Starting column phase...");

    let mut second_highest_found = false;
    let mut second_highest_bid = 0;

    for idx in 0..domain_size {
        // Compute x2 = r2 - col_sum shares
        let mut x2_0_fe = r2_0.clone();
        x2_0_fe.sub(&all_client_s0[idx]);
        
        send_message(&mut stream, &Message::X2Share(fe_to_bytes(&x2_0_fe))).unwrap();
        let Message::X2Share(x2_1_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected X2Share");
        };
        let x2_1_fe = bytes_to_fe(&x2_1_bytes);
        let x2_fe = x2_0_fe.clone() + x2_1_fe.clone();

        // Convert to proper domain range
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

        // MAC check for x2
        let triple_index = num_clients + idx as usize;
        if triple_index < beaver_triples.len() {
            let (a0, a1, b0, b1, c0, c1) = &beaver_triples[triple_index];
            
            let delta_0 = alpha_val2_0.clone() - a0.clone();
            send_message(&mut stream, &Message::DeltaShare(fe_to_bytes(&delta_0))).unwrap();
            let Message::DeltaShare(delta_1_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected DeltaShare");
            };
            let delta_1 = bytes_to_fe(&delta_1_bytes);
            let delta = delta_0.clone() + delta_1.clone();

            let x2_opened = FE::from(x2_val as u32);
            let epsilon_0 = x2_opened.clone() - b0.clone();
            send_message(&mut stream, &Message::EpsilonShare(fe_to_bytes(&epsilon_0))).unwrap();
            let Message::EpsilonShare(epsilon_1_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected EpsilonShare");
            };
            let epsilon_1 = bytes_to_fe(&epsilon_1_bytes);
            let epsilon = epsilon_0.clone() + epsilon_1.clone() - x2_opened;

            let de = delta.clone() * epsilon.clone();
            let mut mac_x2_0 = c0.clone();
            mac_x2_0.add(&(epsilon.clone() * a0.clone()));
            mac_x2_0.add(&(delta.clone() * b0.clone()));
            mac_x2_0.add(&de);

            let mut z_0 = x2_opened.clone();
            z_0.mul(&alpha_val2_0);
            z_0.sub(&mac_x2_0);

            send_message(&mut stream, &Message::ZShare(fe_to_bytes(&z_0))).unwrap();
            let Message::ZShare(z_1_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected ZShare");
            };
            let z_1 = bytes_to_fe(&z_1_bytes);
            let z = z_0.clone() + z_1.clone();
            
            if z.value() != 0 {
                panic!("MAC failure on r2-col_sum opening for idx {}", idx);
            }
        }

        println!("Opened value x2 = r2 - col_sum: {}", x2_val);

        // Shift column values and check for second highest
        let mut col_sum_shifted_val_1_0 = vec![FE::zero(); updated_domain];
        let mut col_sum_shifted_val_2_0 = vec![FE::zero(); updated_domain];
        for i in 0..updated_domain {
            let shift_idx = (i + x2_val as usize) % updated_domain;
            col_sum_shifted_val_1_0[i] = col_values1_0[shift_idx].clone();
            col_sum_shifted_val_2_0[i] = col_values2_0[shift_idx].clone();
        }

        // Exchange shifted values with Party 1 for specific indices
        let mut col_sum_s = FE::zero();
        let mut col_sum_m = FE::zero();
        let n1 = updated_domain - 1;
        let n2 = updated_domain - 2;

        for &j in &[n2, n1] {
            send_message(&mut stream, &Message::ShiftedValues((
                fe_to_bytes(&col_sum_shifted_val_1_0[j]), 
                fe_to_bytes(&col_sum_shifted_val_2_0[j])
            ))).unwrap();
            
            let Message::ShiftedValues((val_1_j, val_2_j)) = receive_message(&mut stream).unwrap() else {
                panic!("Expected ShiftedValues");
            };
            
            let val_1_1_j = bytes_to_fe(&val_1_j);
            let val_2_1_j = bytes_to_fe(&val_2_j);
            
            col_sum_s.add(&col_sum_shifted_val_1_0[j]);
            col_sum_s.add(&val_1_1_j);
            col_sum_m.add(&col_sum_shifted_val_2_0[j]);
            col_sum_m.add(&val_2_1_j);
        }
        
        if col_sum_s.value() >= 1 && !second_highest_found {
            second_highest_found = true;
            second_highest_bid = idx;
            
            // MAC check for second highest bid
            let mut expected_mac = col_sum_s.clone();
            expected_mac.mul(&alpha_val);
            
            if let Err(e) = secure_mac_check(&mut stream, &col_sum_s, &alpha_val2_0, &col_sum_m, true) {
                panic!("MAC failure on second-highest reveal: {}", e);
            }
            
            println!("The value of second highest bid is: {}", second_highest_bid);

            // Find highest bidder
            let mut highest_bidder = 0;
            // Finding the highest bidder
            for bidder in 0..num_clients {
                // Each party computes only their own share contribution
                let mut temp_sum_0 = FE::zero();
                let mut temp_sum_mac_0 = FE::zero();
                
                for index in 0..=idx {
                    let sh_index = (index + x_val[bidder] as usize) % domain_size;
                    temp_sum_0.add(&values1_0[sh_index]);
                    temp_sum_mac_0.add(&values2_0[sh_index]);
                }
                
                // Secure opening protocol for temp_sum
                send_message(&mut stream, &Message::TempSum(fe_to_bytes(&temp_sum_0))).unwrap();
                let Message::TempSum(temp_sum_1_bytes) = receive_message(&mut stream).unwrap() else {
                    panic!("Expected TempSum");
                };
                let temp_sum_1 = bytes_to_fe(&temp_sum_1_bytes);
                let opened_temp_sum = temp_sum_0.clone() + temp_sum_1;
                
                // Secure opening protocol for MAC
                send_message(&mut stream, &Message::MacShare(fe_to_bytes(&temp_sum_mac_0))).unwrap();
                let Message::MacShare(temp_sum_mac_1_bytes) = receive_message(&mut stream).unwrap() else {
                    panic!("Expected MacShare");
                };
                let temp_sum_mac_1 = bytes_to_fe(&temp_sum_mac_1_bytes);
                let opened_temp_sum_mac = temp_sum_mac_0 + temp_sum_mac_1;
                
                // Proper MAC check without reconstructing alpha
                if let Err(e) = secure_mac_check(&mut stream, &opened_temp_sum, &alpha_val_0, &opened_temp_sum_mac, true) {
                    panic!("MAC failure on highest-bidder reveal for bidder {}: {}", bidder, e);
                }
                
                if opened_temp_sum.value() == 0 {
                    println!("Highest bidder found: {}", bidder);
                    send_message(&mut stream, &Message::ColumnResult(second_highest_bid as u64, bidder as u64)).unwrap();
                    return;
                }
            }
            
            send_message(&mut stream, &Message::ColumnResult(second_highest_bid as u64, highest_bidder as u64)).unwrap();
            break;
        }
    }
    
    if !second_highest_found {
        send_message(&mut stream, &Message::Finished).unwrap();
    }

    let client_duration = client_start.elapsed();
    println!("Online time: {:?}", client_duration);
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

fn secure_mac_check(
    stream: &mut TcpStream,
    opened_value: &FE,
    my_alpha_share: &FE,
    my_mac_share: &FE,
    is_party_0: bool,
) -> Result<(), String> {
    // Each party computes their MAC check share: z_i = opened_value * alpha_i - mac_i
    let mut z_my = opened_value.clone();
    z_my.mul(my_alpha_share);
    z_my.sub(my_mac_share);

    if is_party_0 {
        // Party 0 sends first, then receives
        send_message(stream, &Message::ZShare(fe_to_bytes(&z_my))).unwrap();
        let Message::ZShare(z_other_bytes) = receive_message(stream).unwrap() else {
            panic!("Expected ZShare");
        };
        let z_other = bytes_to_fe(&z_other_bytes);
        let z_total = z_my + z_other +  my_mac_share; // TODO: FIX because of using mac directly instead of mac share
        
        if z_total.value() != 0 {
            return Err("MAC check failed".to_string());
        }
    } 
    Ok(())
}
