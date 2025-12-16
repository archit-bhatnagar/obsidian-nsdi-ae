use std::net::TcpStream;
use std::time::Instant;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
use rand::Rng;
use counttree::prg::FromRng;
mod common;
use common::{Message, send_message, receive_message, fe_to_bytes, bytes_to_fe, generate_alpha_shares};

fn secure_mac_check(
    stream: &mut TcpStream,
    opened_value: &FE,
    my_alpha_share: &FE,
    my_mac_share: &FE,
) -> Result<(), String> {
    // Each party computes their MAC check share: z_i = opened_value * alpha_i - mac_i
    let mut z_my = opened_value.clone();
    z_my.mul(my_alpha_share);
    z_my.sub(my_mac_share);

    // Party 1 receives first, then sends
    let Message::ZShare(z_other_bytes) = receive_message(stream).unwrap() else {
        panic!("Expected ZShare");
    };
    let z_other = bytes_to_fe(&z_other_bytes);
    send_message(stream, &Message::ZShare(fe_to_bytes(&z_my))).unwrap();
    let z_total = z_my + z_other + my_mac_share;
    
    if z_total.value() != 0 {
        return Err("MAC check failed".to_string());
    }
    Ok(())
}

fn main() {
    println!("Party 1 (Client) starting...");
    let num_clients = 20;
    let domain_size = 32;

    let mut stream = TcpStream::connect("127.0.0.1:8888").expect("Failed to connect to server");
    println!("Connected to Party 0");

    let overall_start = Instant::now();

    // === Preprocessing Phase ===
    println!("\nMAC Pre-processing and Evaluation:");

    let Message::AlphaShare(alpha_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected alpha share");
    };
    let alpha_val_1 = bytes_to_fe(&alpha_bytes);

    let Message::RShare(r_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected r share");
    };
    let r_1 = bytes_to_fe(&r_bytes);

    let Message::DPFValueShares(values1_1_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected DPFValueShares for values1_1");
    };
    let values1_1: Vec<FE> = values1_1_bytes.iter().map(|b| bytes_to_fe(b)).collect();
    
    let Message::DPFValueShares(values2_1_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected DPFValueShares for values2_1");
    };
    let values2_1: Vec<FE> = values2_1_bytes.iter().map(|b| bytes_to_fe(b)).collect();
    
    let Message::BeaverTripleShares(triple_shares) = receive_message(&mut stream).unwrap() else {
        panic!("Expected Beaver triple shares");
    };
    let mut beaver_triples = Vec::with_capacity(triple_shares.len());
    for (a_bytes, b_bytes, c_bytes) in triple_shares {
        let a1 = bytes_to_fe(&a_bytes);
        let b1 = bytes_to_fe(&b_bytes);
        let c1 = bytes_to_fe(&c_bytes);
        beaver_triples.push((a1, b1, c1));
    }
    
    let Message::AlphaShare(alpha2_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected alpha2 share");
    };
    let alpha_val2_1 = bytes_to_fe(&alpha2_bytes);

    let Message::R2Share(r2_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected r2 share");
    };
    let r2_1 = bytes_to_fe(&r2_bytes);

    let Message::DPFValueShares(col_values1_1_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected DPFValueShares for col_values1_1");
    };
    let col_values1_1: Vec<FE> = col_values1_1_bytes.iter().map(|b| bytes_to_fe(b)).collect();
    
    let Message::DPFValueShares(col_values2_1_bytes) = receive_message(&mut stream).unwrap() else {
        panic!("Expected DPFValueShares for col_values2_1");
    };
    let col_values2_1: Vec<FE> = col_values2_1_bytes.iter().map(|b| bytes_to_fe(b)).collect();

    send_message(&mut stream, &Message::Ready).unwrap();

    let preprocess_time = overall_start.elapsed();
    println!("Pre-processing took: {:?}", preprocess_time);

    let client_start = Instant::now();

    let mut all_client_s1 = vec![FE::zero(); domain_size];
    let mut all_client_m1 = vec![FE::zero(); domain_size];
    let mut x_val = vec![0; num_clients];

    // === Online Phase ===
    for client in 0..num_clients {
        println!("\nClient {}:", client);

        let a_index = rand::thread_rng().gen_range(0, domain_size);
        println!("Secret input (a): {}", a_index);
        let a_val = FE::from(a_index as u32);

        let (_, a_1) = generate_alpha_shares(&a_val);

        send_message(&mut stream, &Message::ClientInput(fe_to_bytes(&a_1))).unwrap();

        // let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);
        let x_share1 = r_1 - a_1;
        let Message::X2Share(x_share0_vec) = receive_message(&mut stream).unwrap() else {
            panic!("Expected XShare");
        };
        let x_share0 = bytes_to_fe(&x_share0_vec);

        let x_opened = (x_share0 + x_share1) ;
        // let x_opened = FE::from(x_val[client] as u32);

        println!("x check is {}", x_opened);

        send_message(&mut stream, &Message::X2Share(fe_to_bytes(&x_share1))).unwrap();

        // MAC check using Beaver triple
        let (a1, b1, c1) = &beaver_triples[client];

        let Message::DeltaShare(delta_0_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected DeltaShare");
        };
        let delta_0 = bytes_to_fe(&delta_0_bytes);

        let delta_1 = alpha_val_1.clone() - a1.clone();
        send_message(&mut stream, &Message::DeltaShare(fe_to_bytes(&delta_1))).unwrap();
        let delta = delta_0.clone() + delta_1.clone();

        let Message::EpsilonShare(epsilon_0_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected EpsilonShare");
        };
        let epsilon_0 = bytes_to_fe(&epsilon_0_bytes);

        let epsilon_1 = x_share1 - b1.clone();
        send_message(&mut stream, &Message::EpsilonShare(fe_to_bytes(&epsilon_1))).unwrap();
        let epsilon = epsilon_0.clone() + epsilon_1.clone();

        let de = delta.clone() * epsilon.clone();

        let mut mac_x_1 = c1.clone();
        mac_x_1.add(&(epsilon.clone() * a1.clone()));
        mac_x_1.add(&(delta.clone() * b1.clone()));

        let mut z_1 = x_opened.clone();
        z_1.mul(&alpha_val_1);
        z_1.sub(&mac_x_1);

        let Message::ZShare(z_0_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected ZShare");
        };
        let z_0 = bytes_to_fe(&z_0_bytes);

        send_message(&mut stream, &Message::ZShare(fe_to_bytes(&z_1))).unwrap();

        let z = z_0.clone() + z_1.clone();
        if z.value() != 0 {
            panic!("MAC failure on r-a opening for client {}", client);
        }

        println!("Opened value x = r - a: {}", x_opened.value());

        // Shift values based on x_opened
        let mut shifted_val_1_1 = vec![FE::zero(); domain_size];
        let mut shifted_val_2_1 = vec![FE::zero(); domain_size];
        for i in 0..domain_size {
            let idx = (i + x_opened.value() as usize) % domain_size;
            shifted_val_1_1[i] = values1_1[idx].clone();
            shifted_val_2_1[i] = values2_1[idx].clone();
        }
        
        // Accumulate values
        let mut cumulative_s1 = FE::zero();
        let mut cumulative_m1 = FE::zero();
        for i in 0..domain_size {
            cumulative_s1.add(&shifted_val_1_1[i]);
            all_client_s1[i].add(&cumulative_s1.clone());
            cumulative_m1.add(&shifted_val_2_1[i]);
            all_client_m1[i].add(&cumulative_m1.clone());
        }
    }

    // === Column Phase ===
    println!("Starting column phase...");
    let updated_domain = num_clients + 1;

    let mut second_highest_found = false;
    let mut second_highest_bid = 0;

    for idx in 0..domain_size {
        // Compute x2 = r2 - col_sum shares
        let mut x2_1_fe = r2_1.clone();
        x2_1_fe.sub(&all_client_s1[idx]);

        let Message::X2Share(x2_0_bytes) = receive_message(&mut stream).unwrap() else {
            panic!("Expected X2Share");
        };
        let x2_0_fe = bytes_to_fe(&x2_0_bytes);

        send_message(&mut stream, &Message::X2Share(fe_to_bytes(&x2_1_fe))).unwrap();

        // Compute x2_val from opened shares
        let x2_fe = x2_0_fe.clone() + x2_1_fe.clone();
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

        // MAC check for x2_val
        let triple_index = num_clients + idx as usize;
        if triple_index < beaver_triples.len() {
            let (a1, b1, c1) = &beaver_triples[triple_index];

            let Message::DeltaShare(delta_0_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected DeltaShare");
            };
            let delta_0 = bytes_to_fe(&delta_0_bytes);

            let delta_1 = alpha_val2_1.clone() - a1.clone();
            send_message(&mut stream, &Message::DeltaShare(fe_to_bytes(&delta_1))).unwrap();

            let Message::EpsilonShare(epsilon_0_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected EpsilonShare");
            };
            let epsilon_0 = bytes_to_fe(&epsilon_0_bytes);

            let x2_opened = FE::from(x2_val as u32);
            let epsilon_1 = x2_opened.clone() - b1.clone();
            send_message(&mut stream, &Message::EpsilonShare(fe_to_bytes(&epsilon_1))).unwrap();
            let epsilon = epsilon_0.clone() + epsilon_1.clone() - x2_opened;

            let delta = delta_0.clone() + delta_1.clone();
            let de = delta * epsilon.clone();

            let mut mac_x2_1 = c1.clone();
            mac_x2_1.add(&(epsilon.clone() * a1.clone()));
            mac_x2_1.add(&(delta.clone() * b1.clone()));

            let mut z_1 = x2_opened.clone();
            z_1.mul(&alpha_val2_1);
            z_1.sub(&mac_x2_1);

            let Message::ZShare(z_0_bytes) = receive_message(&mut stream).unwrap() else {
                panic!("Expected ZShare");
            };
            let z_0 = bytes_to_fe(&z_0_bytes);

            send_message(&mut stream, &Message::ZShare(fe_to_bytes(&z_1))).unwrap();

            let z = z_0.clone() + z_1.clone();
            if z.value() != 0 {
                panic!("MAC failure on r2-col_sum opening for idx {}", idx);
            }
        }

        println!("Opened value x2 = r2 - col_sum: {}", x2_val);

        // Compute shifted column values based on x2_val
        let mut col_sum_shifted_val_1_1 = vec![FE::zero(); updated_domain];
        let mut col_sum_shifted_val_2_1 = vec![FE::zero(); updated_domain];
        for i in 0..updated_domain {
            let shift_idx = (i + x2_val as usize) % updated_domain;
            col_sum_shifted_val_1_1[i] = col_values1_1[shift_idx].clone();
            col_sum_shifted_val_2_1[i] = col_values2_1[shift_idx].clone();
        }

        // Exchange shifted values with Party 0 for specific indices
        let mut col_sum_s = FE::zero();
        let mut col_sum_m = FE::zero();
        let n1 = updated_domain - 1;
        let n2 = updated_domain - 2;

        for &j in &[n2, n1] {
            // Receive Party 0's values for this index
            let Message::ShiftedValues((val_1_0_j, val_2_0_j)) = receive_message(&mut stream).unwrap() else {
                panic!("Expected ShiftedValues");
            };
            
            // Send our values for this index to Party 0
            send_message(&mut stream, &Message::ShiftedValues((
                fe_to_bytes(&col_sum_shifted_val_1_1[j]), 
                fe_to_bytes(&col_sum_shifted_val_2_1[j])
            ))).unwrap();
            
            // Add both parties' values to compute column sum
            let val_1_0_j_fe = bytes_to_fe(&val_1_0_j);
            let val_2_0_j_fe = bytes_to_fe(&val_2_0_j);
            
            col_sum_s.add(&col_sum_shifted_val_1_1[j]);
            col_sum_s.add(&val_1_0_j_fe);
            col_sum_m.add(&col_sum_shifted_val_2_1[j]);
            col_sum_m.add(&val_2_0_j_fe);
        }
        
        // Check if this is the second highest bid
        if col_sum_s.value() >= 1 && !second_highest_found{
            second_highest_found = true;
            second_highest_bid = idx;
            
            // Proper MAC check for second highest bid without reconstructing alpha
            if let Err(e) = secure_mac_check(&mut stream, &col_sum_s, &alpha_val2_1, &col_sum_m) {
                panic!("MAC failure on second-highest reveal: {}", e);
            }
            
            println!("Party 1: Found second highest bid at index: {}", idx);
            
            // Participate in highest bidder computation using secure MPC
            for bidder in 0..num_clients {
                // Each party computes only their own share contribution
                let mut temp_sum_1 = FE::zero();
                let mut temp_sum_mac_1 = FE::zero();
                
                for index in 0..=idx {
                    let sh_index = (index + x_val[bidder] as usize) % domain_size;
                    temp_sum_1.add(&values1_1[sh_index]);
                    temp_sum_mac_1.add(&values2_1[sh_index]);
                }
                
                // Secure opening protocol for temp_sum
                let Message::TempSum(temp_sum_0_bytes) = receive_message(&mut stream).unwrap() else {
                    panic!("Expected TempSum");
                };
                let temp_sum_0 = bytes_to_fe(&temp_sum_0_bytes);
                send_message(&mut stream, &Message::TempSum(fe_to_bytes(&temp_sum_1))).unwrap();
                let opened_temp_sum = temp_sum_0 + temp_sum_1.clone();
                
                // Secure opening protocol for MAC
                let Message::MacShare(temp_sum_mac_0_bytes) = receive_message(&mut stream).unwrap() else {
                    panic!("Expected MacShare");
                };
                let temp_sum_mac_0 = bytes_to_fe(&temp_sum_mac_0_bytes);
                send_message(&mut stream, &Message::MacShare(fe_to_bytes(&temp_sum_mac_1))).unwrap();
                let opened_temp_sum_mac = temp_sum_mac_0 + temp_sum_mac_1;
                
                // Proper MAC check without reconstructing alpha
                if let Err(e) = secure_mac_check(&mut stream, &opened_temp_sum, &alpha_val_1, &opened_temp_sum_mac) {
                    panic!("MAC failure on highest-bidder reveal for bidder {}: {}", bidder, e);
                }
                
                if opened_temp_sum.value() == 0 {
                    println!("Party 1: Highest bidder found: {}", bidder);
                }
            }
            
            break;
        }

        println!("Party 1: Processed column index: {}", idx);
    }

    // Receive final result confirmation from Party 0
    let result = receive_message(&mut stream).unwrap();
    match result {
        Message::ColumnResult(second_highest, winner) => {
            println!("Result confirmed: Second highest bid = {}, Winner = {}", second_highest, winner);
        },
        Message::Finished => {
            println!("No winner found");
        },
        _ => {
            panic!("Unexpected message received");
        }
    }

    let client_duration = client_start.elapsed();
    println!("Online time: {:?}", client_duration);

    let total_time = overall_start.elapsed();
    println!("Total execution time: {:?}", total_time);
}
