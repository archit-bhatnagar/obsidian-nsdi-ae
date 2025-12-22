// party1.rs
use std::{net::TcpStream, time::Instant, thread, sync::mpsc};
use std::time::Duration;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
mod common;
use common::{Message, send_message, receive_message, fe_to_bytes, bytes_to_fe, bulk_fe_to_bytes, bulk_bytes_to_fe};
use chrono::Local;


#[derive(Debug)]
enum SendCommand {
    Send(Message),
    Close,
}

#[derive(Debug)]  
enum ReceiveCommand {
    Receive,
    Close,
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

fn main() {
    println!("Party 1 (Client) starting...");
    
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let num_clients = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };
    let domain_size = if args.len() > 2 {
        args[2].parse().unwrap_or(1024)
    } else {
        1024
    };
    
    println!("Configuration: {} clients, domain size {}", num_clients, domain_size);

    let stream = TcpStream::connect("127.0.0.1:8889").expect("Failed to connect to server");
    // println!("Connected to Party 0");

    stream.set_nodelay(true);

    let send_stream = stream.try_clone().expect("Failed to clone stream for sending");
    let receive_stream = stream;


    let (send_tx, send_rx) = mpsc::channel::<SendCommand>();
    let (receive_tx, receive_rx) = mpsc::channel::<ReceiveCommand>();
    let (response_tx, response_rx) = mpsc::channel::<Message>();

    let send_handle = thread::spawn(move || {
        let mut send_stream = send_stream;
        while let Ok(command) = send_rx.recv() {
            match command {
                SendCommand::Send(message) => {
                    if let Err(e) = send_message(&mut send_stream, &message) {
                        eprintln!("Send error: {:?}", e);
                        break;
                    }
                }
                SendCommand::Close => break,
            }
        }
        println!("Send thread terminated");
    });

    let receive_handle = thread::spawn(move || {
        let mut receive_stream = receive_stream;
        while let Ok(command) = receive_rx.recv() {
            match command {
                ReceiveCommand::Receive => {
                    match receive_message(&mut receive_stream) {
                        Ok(message) => {
                            if response_tx.send(message).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Receive error: {:?}", e);
                            break;
                        }
                    }
                }
                ReceiveCommand::Close => break,
            }
        }
        println!("Receive thread terminated");
    });

    let overall_start = Instant::now();

    // Timing variables
    let mut preprocessing_time = Duration::new(0, 0);
    let mut client_processing_time = Duration::new(0, 0);
    let mut round1_compute_time = Duration::new(0, 0);
    let mut round1_comm_time = Duration::new(0, 0);
    let mut round1_process_time = Duration::new(0, 0);
    let mut round2_compute_time = Duration::new(0, 0);
    let mut round2_comm_time = Duration::new(0, 0);
    let mut round4_comm_time = Duration::new(0, 0);
    let mut round5_compute_time = Duration::new(0, 0);
    let mut round5_comm_time = Duration::new(0, 0);
    let mut round6_compute_time = Duration::new(0, 0);
    let mut round6_comm_time = Duration::new(0, 0);
    let mut mac_verification_time = Duration::new(0, 0);
    
    // Communication tracking
    let mut preprocessing_comm_size = 0usize;
    let mut online_comm_size = 0usize;

    // === Preprocessing Phase ===
    // println!("Receiving preprocessing data...");
    let preprocess_start = Instant::now();
    
    receive_tx.send(ReceiveCommand::Receive).unwrap();
    let preprocessing_response = response_rx.recv().unwrap();
    
    // Track preprocessing communication (received from party0)
    let preprocessing_recv_size = bincode::serialize(&preprocessing_response).unwrap().len();
    preprocessing_comm_size += preprocessing_recv_size;
    
    let Message::PreprocessingData {
        alpha_share,
        r_share,
        r2_share,
        r3_share,
        values1_shares_bulk,        // OPTIMIZED: bulk arrays
        values2_shares_bulk,        // OPTIMIZED: bulk arrays  
        col_values1_shares_bulk,    // OPTIMIZED: bulk arrays
        col_values2_shares_bulk,    // OPTIMIZED: bulk arrays
        tie_values1_shares_bulk,    // OPTIMIZED: bulk arrays
        tie_values2_shares_bulk,    // OPTIMIZED: bulk arrays
        alpha_r2_share,
        alpha_r3_share,
        x_values,
    } = preprocessing_response else {
        panic!("Expected PreprocessingData");
    };

    // OPTIMIZED: Bulk deserialization for preprocessing
    // println!("Deserializing preprocessing data with bulk optimization...");
    let deserialize_start = Instant::now();
    let alpha_val_1 = bytes_to_fe(&alpha_share);
    let r_1 = bytes_to_fe(&r_share);
    let r2_1 = bytes_to_fe(&r2_share);
    let r3_1 = bytes_to_fe(&r3_share);
    let values1_1 = bulk_bytes_to_fe(&values1_shares_bulk);        // OPTIMIZED
    let values2_1 = bulk_bytes_to_fe(&values2_shares_bulk);        // OPTIMIZED
    let col_sum_values1_1 = bulk_bytes_to_fe(&col_values1_shares_bulk); // OPTIMIZED
    let col_sum_values2_1 = bulk_bytes_to_fe(&col_values2_shares_bulk); // OPTIMIZED
    let tie_values1_1 = bulk_bytes_to_fe(&tie_values1_shares_bulk); // OPTIMIZED
    let tie_values2_1 = bulk_bytes_to_fe(&tie_values2_shares_bulk); // OPTIMIZED
    let alpha_r2_1 = bytes_to_fe(&alpha_r2_share);
    let alpha_r3_1 = bytes_to_fe(&alpha_r3_share);
    // println!("Preprocessing deserialization took: {:?}", deserialize_start.elapsed());

    send_tx.send(SendCommand::Send(Message::Ready)).unwrap();

    preprocessing_time = preprocess_start.elapsed();
    println!("Pre-processing took: {:?}", preprocessing_time);

    let client_start = Instant::now();
    let updated_domain = num_clients + 1;
    let max_possible_sum = domain_size;

    // === Parallel Client Processing ===
    // println!("Computing client cumulative sums in parallel...");
    let client_processing_start = Instant::now();
    
    let values1_1_clone = values1_1.clone();
    let values2_1_clone = values2_1.clone();
    let x_values_clone = x_values.clone();
    
    let client_processing_handle = thread::spawn(move || {
        let mut all_client_s1 = vec![FE::zero(); domain_size];
        let mut all_client_m1 = vec![FE::zero(); domain_size];

        let batch_size = num_clients / 4;
        let mut handles = vec![];
        
        for batch_start in (0..num_clients).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(num_clients);
            let values1_1_batch = values1_1_clone.clone();
            let values2_1_batch = values2_1_clone.clone();
            let x_values_batch = x_values_clone[batch_start..batch_end].to_vec();
            
            let handle = thread::spawn(move || {
                let mut batch_s = vec![FE::zero(); domain_size];
                let mut batch_m = vec![FE::zero(); domain_size];
                
                for &client_x in x_values_batch.iter() {
                    let mut shifted_val_1_1 = vec![FE::zero(); domain_size];
                    let mut shifted_val_2_1 = vec![FE::zero(); domain_size];
                    
                    for i in 0..domain_size {
                        let idx = (i + client_x as usize) % domain_size;
                        shifted_val_1_1[i] = values1_1_batch[idx].clone();
                        shifted_val_2_1[i] = values2_1_batch[idx].clone();
                    }

                    let mut cumulative_s1 = FE::zero();
                    let mut cumulative_m1 = FE::zero();
                    
                    for i in 0..domain_size {
                        cumulative_s1.add(&shifted_val_1_1[i]);
                        batch_s[i].add(&cumulative_s1.clone());
                        cumulative_m1.add(&shifted_val_2_1[i]);
                        batch_m[i].add(&cumulative_m1.clone());
                    }
                }
                (batch_s, batch_m)
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let (batch_s, batch_m) = handle.join().unwrap();
            for i in 0..domain_size {
                all_client_s1[i].add(&batch_s[i]);
                all_client_m1[i].add(&batch_m[i]);
            }
        }
        
        (all_client_s1, all_client_m1)
    });

    let (all_client_s1, all_client_m1) = client_processing_handle.join().unwrap();
    client_processing_time = client_processing_start.elapsed();

    let online_start = Instant::now();
    let mut all_opened_values = Vec::new();
    let mut all_mac_shares_1 = Vec::new();

    // === ROUND 1 - OPTIMIZED ===
    // println!("Round 1: Opening x2 values");

    // let mut now = Local::now();
    // println!("r1 compute start: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));
    
    let round1_compute_start = Instant::now();
    let x2_computation_handle = thread::spawn(move || {
        let mut x2_shares_1 = Vec::with_capacity(domain_size);
        for idx in 0..domain_size {
            let x2_1_fe = r2_1.clone() - all_client_s1[idx].clone();
            x2_shares_1.push(x2_1_fe);
        }
        x2_shares_1
    });
    let x2_shares_1 = x2_computation_handle.join().unwrap();
    
    round1_compute_time = round1_compute_start.elapsed();
    // now = Local::now();
    // println!("r1 compute end: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));

    receive_tx.send(ReceiveCommand::Receive).unwrap();

    let round1_comm_start = Instant::now();
    let round1_response = response_rx.recv().unwrap();
    
    let Message::Round1X2Opening { x2_shares_bulk: x2_shares_0_bulk } = round1_response else {
        panic!("Expected Round1X2Opening");
    };
    // println!("🟨 PARTY1: Received {} bytes of x2 data", x2_shares_0_bulk.len());
    // println!("🟨 PARTY1: First 8 bytes of received data: {:?}", &x2_shares_0_bulk[0..8.min(x2_shares_0_bulk.len())]);


    let x2_shares_0 = bulk_bytes_to_fe(&x2_shares_0_bulk);
    // println!("🟨 PARTY1: Deserialized x2_shares_0[0] = {}", x2_shares_0[0].value());
    // println!("🟨 PARTY1: Computed x2_shares_1[0] = {}", x2_shares_1[0].value());

    // now = Local::now();
    // println!("r1 recv: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));

    // OPTIMIZED: Send Party 1's x2 shares as bulk
    let serialize_start = Instant::now();
    let round1_msg = Message::Round1X2Opening {
        x2_shares_bulk: bulk_fe_to_bytes(&x2_shares_1),  // OPTIMIZED
    };
    let round1_send_size = bincode::serialize(&round1_msg).unwrap().len();
    online_comm_size += round1_send_size;
    // println!("Round 1 serialization took: {:?}", serialize_start.elapsed());
    send_tx.send(SendCommand::Send(round1_msg)).unwrap();

    // now = Local::now();
    // println!("r1 send: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));

    // OPTIMIZED: Process x2 with bulk deserialization
    let round1_process_start = Instant::now();
    let deserialize_start = Instant::now();
    let x2_shares_0 = bulk_bytes_to_fe(&x2_shares_0_bulk);  // OPTIMIZED
    // println!("x2_shares is {}", x2_shares_0[0]);
    // println!("Round 1 deserialization took: {:?}", deserialize_start.elapsed());
    
    round1_comm_time = round1_comm_start.elapsed();
    
    let col_values_clone = (col_sum_values1_1.clone(), col_sum_values2_1.clone());
    let alpha_r2_1_clone = alpha_r2_1.clone();
    let all_client_m1_clone = all_client_m1.clone();
    
    let x2_processing_handle = thread::spawn(move || {
        let mut all_col_shifted_values = Vec::new();
        let mut opened_values = Vec::new();
        let mut mac_shares = Vec::new();
        
        for (idx, x2_0_fe) in x2_shares_0.iter().enumerate() {
            let x2_fe = x2_0_fe.clone() + x2_shares_1[idx].clone();
            // println!(" The x2 {:?}", x2_fe.value());
            
            opened_values.push(fe_to_bytes(&x2_fe));
            let alpha_x2_1 = alpha_r2_1_clone.clone() - all_client_m1_clone[idx].clone();
            mac_shares.push(fe_to_bytes(&alpha_x2_1));
            
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

            
            let mut col_sum_shifted_val_1_1 = vec![FE::zero(); updated_domain];
            let mut col_sum_shifted_val_2_1 = vec![FE::zero(); updated_domain];
            
            for i in 0..updated_domain {
                let shift_idx = (i + x2_val as usize) % updated_domain;
                col_sum_shifted_val_1_1[i] = col_values_clone.0[shift_idx].clone();
                col_sum_shifted_val_2_1[i] = col_values_clone.1[shift_idx].clone();
            }
            
            all_col_shifted_values.push((col_sum_shifted_val_1_1, col_sum_shifted_val_2_1));
        }
        
        (all_col_shifted_values, opened_values, mac_shares)
    });

    let (all_col_shifted_values, mut round1_opened, mut round1_mac) = x2_processing_handle.join().unwrap();
    all_opened_values.append(&mut round1_opened);
    all_mac_shares_1.append(&mut round1_mac);
    round1_process_time = round1_process_start.elapsed();

    let mut current_threshold = num_clients - 1;
    let mut second_highest_found = false;

    while current_threshold > 0 && !second_highest_found {
        println!("Checking for threshold: {} bidders", current_threshold);
        
        // ROUND 2
        let round2_compute_start = Instant::now();
        let mut threshold_sum_1 = FE::zero();
        let mut threshold_mac_1 = FE::zero();
        
        for idx in 0..domain_size {
            let (ref col_1_1, ref col_2_1) = &all_col_shifted_values[idx];
            threshold_sum_1.add(&col_1_1[current_threshold]);
            threshold_mac_1.add(&col_2_1[current_threshold]);
        }
        

        let r3_shift_1 = r3_1.clone() - threshold_sum_1.clone();
        let alpha_r3_threshold_1 = alpha_r3_1.clone() - threshold_mac_1.clone();
        all_mac_shares_1.push(fe_to_bytes(&alpha_r3_threshold_1));
        round2_compute_time += round2_compute_start.elapsed();
        
        let round2_comm_start = Instant::now();
        receive_tx.send(ReceiveCommand::Receive).unwrap();
        
        let round2_response = response_rx.recv().unwrap();
        let Message::Round3R3Shift { .. } = round2_response else {
            panic!("Expected Round3R3Shift");
        };
        
        let round2_msg = Message::Round3R3Shift {
            r3_shift_share: fe_to_bytes(&r3_shift_1),
        };
        let round2_send_size = bincode::serialize(&round2_msg).unwrap().len();
        online_comm_size += round2_send_size;
        send_tx.send(SendCommand::Send(round2_msg)).unwrap();
        round2_comm_time += round2_comm_start.elapsed();

        // Simplified tie detection for benchmarking
        let tie_shifting_handle = thread::spawn(move || {
            let mut tie_shifted_val_1_1 = vec![FE::zero(); max_possible_sum];
            let mut tie_shifted_val_2_1 = vec![FE::zero(); max_possible_sum];
            
            if !tie_values1_1.is_empty() {
                tie_shifted_val_1_1[0] = tie_values1_1[0].clone();
                tie_shifted_val_2_1[0] = tie_values2_1[0].clone();
            }
            
            (tie_shifted_val_1_1, tie_shifted_val_2_1)
        });

        let (tie_shifted_val_1_1, tie_shifted_val_2_1) = tie_shifting_handle.join().unwrap();

        // ROUND 4
        let round4_comm_start = Instant::now();
        receive_tx.send(ReceiveCommand::Receive).unwrap();
        
        let exact_one_check_1 = tie_shifted_val_1_1[0].clone();
        all_mac_shares_1.push(fe_to_bytes(&tie_shifted_val_2_1[0]));
        
        let round4_response = response_rx.recv().unwrap();
        let Message::Round4TieResult { .. } = round4_response else {
            panic!("Expected Round4TieResult");
        };
        
        let round4_msg = Message::Round4TieResult {
            tie_result_share: fe_to_bytes(&exact_one_check_1),
        };
        let round4_send_size = bincode::serialize(&round4_msg).unwrap().len();
        online_comm_size += round4_send_size;
        send_tx.send(SendCommand::Send(round4_msg)).unwrap();
        round4_comm_time += round4_comm_start.elapsed();
        
        second_highest_found = true;

        // ROUND 5 - OPTIMIZED
        let round5_compute_start = Instant::now();
        let col_computation_handle = thread::spawn(move || {
            let mut col_ge_threshold_shares_1 = Vec::with_capacity(domain_size);
            let mut mac_accum_shares_1 = Vec::with_capacity(domain_size);

            for idx in 0..domain_size {
                let (ref col_1_1, ref col_2_1) = &all_col_shifted_values[idx];
                
                let mut col_ge_threshold_1 = FE::zero();
                let mut mac_accum_1 = FE::zero();
                
                for j in current_threshold..updated_domain {
                    col_ge_threshold_1.add(&col_1_1[j]);
                    mac_accum_1.add(&col_2_1[j]);
                }
                
                col_ge_threshold_shares_1.push(col_ge_threshold_1);
                mac_accum_shares_1.push(mac_accum_1);
            }
            (col_ge_threshold_shares_1, mac_accum_shares_1)
        });

        let (col_ge_threshold_shares_1, mac_accum_shares_1) = col_computation_handle.join().unwrap();
        round5_compute_time = round5_compute_start.elapsed();

        let round5_comm_start = Instant::now();
        receive_tx.send(ReceiveCommand::Receive).unwrap();

        let round5_response = response_rx.recv().unwrap();
        let Message::Round5SecondHighest { col_ge_threshold_shares_bulk: col_ge_threshold_shares_0_bulk, .. } = round5_response else {
            panic!("Expected Round5SecondHighest");
        };

        // OPTIMIZED: Bulk serialization for Round 5
        let serialize_start = Instant::now();
        let round5_msg = Message::Round5SecondHighest {
            found_second_highest: true,
            second_highest_bid: 0,
            col_ge_threshold_shares_bulk: bulk_fe_to_bytes(&col_ge_threshold_shares_1), // OPTIMIZED
        };
        let round5_send_size = bincode::serialize(&round5_msg).unwrap().len();
        online_comm_size += round5_send_size;
        // println!("Round 5 serialization took: {:?}", serialize_start.elapsed());
        send_tx.send(SendCommand::Send(round5_msg)).unwrap();
        round5_comm_time = round5_comm_start.elapsed();

        // OPTIMIZED: Bulk deserialization for Round 5
        let deserialize_start = Instant::now();
        let col_ge_threshold_shares_0 = bulk_bytes_to_fe(&col_ge_threshold_shares_0_bulk); // OPTIMIZED
        // println!("Round 5 deserialization took: {:?}", deserialize_start.elapsed());

        let mut second_highest_bid = 0;
        for idx in 0..domain_size {
            let col_ge_threshold = col_ge_threshold_shares_0[idx].clone() + col_ge_threshold_shares_1[idx].clone();
            
            if col_ge_threshold.value() >= 1 {
                second_highest_bid = idx;
                all_opened_values.push(fe_to_bytes(&col_ge_threshold));
                all_mac_shares_1.push(fe_to_bytes(&mac_accum_shares_1[idx]));
                break;
            }
        }

        // ROUND 6 - OPTIMIZED
        let round6_compute_start = Instant::now();
        let winner_computation_handle = thread::spawn(move || {
            let mut temp_sum_shares_1 = Vec::with_capacity(num_clients);
            let mut temp_sum_mac_shares_1 = Vec::with_capacity(num_clients);

            for bidder in 0..num_clients {
                let mut temp_sum_1 = FE::zero();
                let mut temp_sum_mac_1 = FE::zero();
                
                for index in 0..=second_highest_bid {
                    let sh_index = (index + x_values[bidder] as usize) % domain_size;
                    temp_sum_1.add(&values1_1[sh_index]);
                    temp_sum_mac_1.add(&values2_1[sh_index]);
                }
                
                temp_sum_shares_1.push(temp_sum_1);
                temp_sum_mac_shares_1.push(temp_sum_mac_1);
            }
            (temp_sum_shares_1, temp_sum_mac_shares_1)
        });

        let (temp_sum_shares_1, temp_sum_mac_shares_1) = winner_computation_handle.join().unwrap();
        round6_compute_time = round6_compute_start.elapsed();

        let round6_comm_start = Instant::now();
        receive_tx.send(ReceiveCommand::Receive).unwrap();

        let round6_response = response_rx.recv().unwrap();
        let Message::Round6Winner { temp_sum_shares_bulk: temp_sum_shares_0_bulk } = round6_response else {
            panic!("Expected Round6Winner");
        };

        // OPTIMIZED: Bulk serialization for Round 6
        let serialize_start = Instant::now();
        let round6_msg = Message::Round6Winner {
            temp_sum_shares_bulk: bulk_fe_to_bytes(&temp_sum_shares_1), // OPTIMIZED
        };
        let round6_send_size = bincode::serialize(&round6_msg).unwrap().len();
        online_comm_size += round6_send_size;
        // println!("Round 6 serialization took: {:?}", serialize_start.elapsed());
        send_tx.send(SendCommand::Send(round6_msg)).unwrap();
        round6_comm_time = round6_comm_start.elapsed();

        // OPTIMIZED: Bulk deserialization for Round 6
        let deserialize_start = Instant::now();
        let temp_sum_shares_0 = bulk_bytes_to_fe(&temp_sum_shares_0_bulk); // OPTIMIZED
        // println!("Round 6 deserialization took: {:?}", deserialize_start.elapsed());

        let mut highest_bidder = 0;
        for bidder in 0..num_clients {
            let temp_sum_total = temp_sum_shares_0[bidder].clone() + temp_sum_shares_1[bidder].clone();
            
            all_opened_values.push(fe_to_bytes(&temp_sum_total));
            all_mac_shares_1.push(fe_to_bytes(&temp_sum_mac_shares_1[bidder]));
            
            if temp_sum_total.value() == 0 {
                highest_bidder = bidder;
            }
        }

        // MAC VERIFICATION - OPTIMIZED
        let mac_start = Instant::now();
        receive_tx.send(ReceiveCommand::Receive).unwrap();
        
        let final_response = response_rx.recv().unwrap();
        let Message::FinalMacVerification { alpha_share: alpha_0_bytes, all_mac_shares_bulk: mac_shares_0_bulk, .. } = final_response else {
            panic!("Expected FinalMacVerification");
        };

        let alpha_0 = bytes_to_fe(&alpha_0_bytes);

        // OPTIMIZED: Bulk serialization for MAC verification
        let serialize_start = Instant::now();
        let final_msg = Message::FinalMacVerification {
            alpha_share: fe_to_bytes(&alpha_val_1),
            all_opened_values_bulk: all_opened_values.iter().flat_map(|v| v.iter()).cloned().collect(), // OPTIMIZED
            all_mac_shares_bulk: all_mac_shares_1.iter().flat_map(|v| v.iter()).cloned().collect(),    // OPTIMIZED
        };
        let mac_send_size = bincode::serialize(&final_msg).unwrap().len();
        online_comm_size += mac_send_size;
        // println!("MAC verification serialization took: {:?}", serialize_start.elapsed());
        send_tx.send(SendCommand::Send(final_msg)).unwrap();

        // OPTIMIZED: Bulk deserialization for MAC verification
        let deserialize_start = Instant::now();
        let mac_shares_0_vec: Vec<Vec<u8>> = mac_shares_0_bulk.chunks_exact(8).map(|chunk| chunk.to_vec()).collect(); // OPTIMIZED
        // println!("MAC verification deserialization took: {:?}", deserialize_start.elapsed());

        let opened_values_for_verification = all_opened_values.clone();
        let mac_shares_1_for_verification = all_mac_shares_1.clone();
        let num_checks = opened_values_for_verification.len();

        let mac_verification_handle = thread::spawn(move || {
            let alpha_reconstructed = alpha_0 + alpha_val_1;
            let mut all_passed = true;
            
            for i in 0..opened_values_for_verification.len() {
                let opened_value = bytes_to_fe(&opened_values_for_verification[i]);
                let mac_0 = bytes_to_fe(&mac_shares_0_vec[i]);
                let mac_1 = bytes_to_fe(&mac_shares_1_for_verification[i]);
                
                let mut z_0 = opened_value.clone();
                z_0.mul(&alpha_0);
                z_0.sub(&mac_0);
                
                let mut z_1 = opened_value.clone();
                z_1.mul(&alpha_val_1);
                z_1.sub(&mac_1);
                
                let z_total = z_0 + z_1;
                if z_total.value() != 0 {
                    all_passed = false;
                    break;
                }
            }
            (alpha_reconstructed, all_passed)
        });

        let (alpha_reconstructed, mac_passed) = mac_verification_handle.join().unwrap();
        mac_verification_time = mac_start.elapsed();

        if mac_passed {
            println!("All {} MAC checks passed on Party 1!", num_checks);
        } else {
            println!("Some MAC checks failed!");
        }

        println!("Alpha reconstructed: {}", alpha_reconstructed.value());

        receive_tx.send(ReceiveCommand::Receive).unwrap();
        let result = response_rx.recv().unwrap();
        let Message::Result(second_highest, winner) = result else {
            panic!("Expected Result");
        };

        println!("Second highest bid: {}", second_highest);
        println!("Highest bidder: {}", winner);
        break;
    }

    send_tx.send(SendCommand::Close).unwrap();
    receive_tx.send(ReceiveCommand::Close).unwrap();
    
    let _ = send_handle.join();
    let _ = receive_handle.join();

    let online_time = online_start.elapsed();
    let total_time = overall_start.elapsed();

    // TIMING BREAKDOWN
    println!("\n=== OPTIMIZED TIMING BREAKDOWN ===");
    println!("Domain size: {}, Clients: {}", domain_size, num_clients);
    println!("Preprocessing:       {:?}", preprocessing_time);
    println!("Client processing:   {:?}", client_processing_time);
    println!("Round 1 compute:     {:?}", round1_compute_time);
    println!("Round 1 comm:        {:?}", round1_comm_time);
    println!("Round 1 process:     {:?}", round1_process_time);
    println!("Round 2 compute:     {:?}", round2_compute_time);
    println!("Round 2 comm:        {:?}", round2_comm_time);
    println!("Round 4 comm:        {:?}", round4_comm_time);
    println!("Round 5 compute:     {:?}", round5_compute_time);
    println!("Round 5 comm:        {:?}", round5_comm_time);
    println!("Round 6 compute:     {:?}", round6_compute_time);
    println!("Round 6 comm:        {:?}", round6_comm_time);
    println!("MAC verification:    {:?}", mac_verification_time);
    
    let total_compute = preprocessing_time + client_processing_time + round1_compute_time + round1_process_time + round2_compute_time + round5_compute_time + round6_compute_time + mac_verification_time;
    let total_comm = round1_comm_time + round2_comm_time + round4_comm_time + round5_comm_time + round6_comm_time;
    
    println!("Total compute:       {:?} ({:.1}%)", total_compute, total_compute.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("Total comm:          {:?} ({:.1}%)", total_comm, total_comm.as_secs_f64() / total_time.as_secs_f64() * 100.0);
    println!("Online time:         {:?}", online_time);
    println!("Total time:          {:?}", total_time);
    
    // Parseable summary for scripts
    println!("\n=== BENCHMARK SUMMARY ===");
    println!("PREPROCESS_TIME_MS: {:.3}", preprocessing_time.as_secs_f64() * 1000.0);
    println!("ONLINE_TIME_MS: {:.3}", online_time.as_secs_f64() * 1000.0);
    println!("PREPROCESS_COMM_BYTES: {}", preprocessing_comm_size);
    println!("ONLINE_COMM_BYTES: {}", online_comm_size);
}
