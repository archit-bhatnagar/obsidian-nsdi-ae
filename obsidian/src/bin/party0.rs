// party0.rs
use std::{net::TcpListener, time::Instant, thread, sync::mpsc};
use std::time::Duration;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*;
use rand::Rng;
use counttree::prg::FromRng;
mod common;
use common::{Message, send_message, receive_message, fe_to_bytes, bytes_to_fe, generate_alpha_shares, bulk_fe_to_bytes, bulk_bytes_to_fe};
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

fn preprocess_mac(
    domain_size: usize,
    alpha_val: &FE,
) -> ((SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), (SketchDPFKey<FE, FE>, SketchDPFKey<FE, FE>), FE, (FE, FE), (FE, FE)) {
    let mut rng = rand::thread_rng();
    let r_usize = rng.gen_range(0, domain_size);
    let nbits = (domain_size as f64).log2().ceil() as u8;
    let r: FE = FE::from(r_usize as u32);
    
    let (r0, r1) = generate_alpha_shares(&r);
    let (alpha0, alpha1) = generate_alpha_shares(alpha_val);
    
    let alpha = u32_to_bits(nbits, r_usize as u32);
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

fn main() {
    println!("Party 0 (Server) starting...");
    
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

    let listener = TcpListener::bind("127.0.0.1:8889").expect("Failed to bind to address");
    // println!("Listening on port 8888");
    let (stream, _) = listener.accept().expect("Failed to accept connection");
    // println!("Party 1 connected");

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

    // println!("Starting preprocessing...");
    let preprocess_start = Instant::now();
    let alpha_val = FE::random();
    
    let fss1_handle = thread::spawn(move || {
        preprocess_mac(domain_size, &alpha_val)
    });
    
    let alpha_val_clone = alpha_val.clone();
    let updated_domain = num_clients + 1;
    let fss2_handle = thread::spawn(move || {
        preprocess_mac(updated_domain, &alpha_val_clone)  
    });
    
    let alpha_val_clone2 = alpha_val.clone();
    let max_possible_sum = domain_size;
    let fss3_handle = thread::spawn(move || {
        preprocess_mac(max_possible_sum, &alpha_val_clone2)
    });

    let ((key1_0, key1_1), (key2_0, key2_1), _r, (r_0, r_1), (alpha_val_0, alpha_val_1)) = 
        fss1_handle.join().unwrap();
    let ((col_key1_0, col_key1_1), (col_key2_0, col_key2_1), r2, (r2_0, r2_1), _) = 
        fss2_handle.join().unwrap();
    let ((tie_key1_0, tie_key1_1), (tie_key2_0, tie_key2_1), r3, (r3_0, r3_1), _) = 
        fss3_handle.join().unwrap();

    let eval1_handle = thread::spawn(move || {
        (eval_all(&key1_0, domain_size), eval_all(&key1_1, domain_size),
         eval_all(&key2_0, domain_size), eval_all(&key2_1, domain_size))
    });
    
    let eval2_handle = thread::spawn(move || {
        (eval_all(&col_key1_0, updated_domain), eval_all(&col_key1_1, updated_domain),
         eval_all(&col_key2_0, updated_domain), eval_all(&col_key2_1, updated_domain))
    });
    
    let eval3_handle = thread::spawn(move || {
        (eval_all(&tie_key1_0, max_possible_sum), eval_all(&tie_key1_1, max_possible_sum),
         eval_all(&tie_key2_0, max_possible_sum), eval_all(&tie_key2_1, max_possible_sum))
    });

    let (values1_0, values1_1, values2_0, values2_1) = eval1_handle.join().unwrap();
    let (col_sum_values1_0, col_sum_values1_1, col_sum_values2_0, col_sum_values2_1) = eval2_handle.join().unwrap();
    let (tie_values1_0, tie_values1_1, tie_values2_0, tie_values2_1) = eval3_handle.join().unwrap();

    let alpha_r2 = alpha_val.clone() * r2.clone();
    let (alpha_r2_0, alpha_r2_1) = generate_alpha_shares(&alpha_r2);
    let alpha_r3 = alpha_val.clone() * r3.clone();
    let (alpha_r3_0, alpha_r3_1) = generate_alpha_shares(&alpha_r3);

    let mut x_val = vec![0; num_clients];
    for client in 0..num_clients {
        let a_index = rand::thread_rng().gen_range(0, domain_size);
        let a_val = FE::from(a_index as u32);
        let (a_0, a_1) = generate_alpha_shares(&a_val);
        let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
        let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);
        x_val[client] = (x_share0 + x_share1) % (domain_size as u64);
    }

    preprocessing_time = preprocess_start.elapsed();

    // OPTIMIZED: Bulk serialization for preprocessing
    // println!("Serializing preprocessing data with bulk optimization...");
    let serialize_start = Instant::now();
    let preprocessing_msg = Message::PreprocessingData {
        alpha_share: fe_to_bytes(&alpha_val_1),
        r_share: fe_to_bytes(&r_1),
        r2_share: fe_to_bytes(&r2_1),
        r3_share: fe_to_bytes(&r3_1),
        values1_shares_bulk: bulk_fe_to_bytes(&values1_1),        // OPTIMIZED
        values2_shares_bulk: bulk_fe_to_bytes(&values2_1),        // OPTIMIZED
        col_values1_shares_bulk: bulk_fe_to_bytes(&col_sum_values1_1), // OPTIMIZED
        col_values2_shares_bulk: bulk_fe_to_bytes(&col_sum_values2_1), // OPTIMIZED
        tie_values1_shares_bulk: bulk_fe_to_bytes(&tie_values1_1), // OPTIMIZED
        tie_values2_shares_bulk: bulk_fe_to_bytes(&tie_values2_1), // OPTIMIZED
        alpha_r2_share: fe_to_bytes(&alpha_r2_1),
        alpha_r3_share: fe_to_bytes(&alpha_r3_1),
        x_values: x_val.clone(),
    };
    let preprocessing_send_size = bincode::serialize(&preprocessing_msg).unwrap().len();
    preprocessing_comm_size += preprocessing_send_size;
    // println!("Preprocessing serialization took: {:?}", serialize_start.elapsed());
    // println!("📤 Preprocessing SEND: {} bytes", preprocessing_send_size);

    send_tx.send(SendCommand::Send(preprocessing_msg)).unwrap();
    receive_tx.send(ReceiveCommand::Receive).unwrap();

    let client_processing_start = Instant::now();
    let values1_0_clone = values1_0.clone();
    let values2_0_clone = values2_0.clone();
    let x_val_clone = x_val.clone();
    println!("x_val is {}",x_val[0]);
    
    let client_processing_handle = thread::spawn(move || {
        let mut all_client_s0 = vec![FE::zero(); domain_size];
        let mut all_client_m0 = vec![FE::zero(); domain_size];

        let batch_size = num_clients / 4;
        let mut handles = vec![];
        
        for batch_start in (0..num_clients).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(num_clients);
            let values1_0_batch = values1_0_clone.clone();
            let values2_0_batch = values2_0_clone.clone();
            let x_val_batch = x_val_clone[batch_start..batch_end].to_vec();
            
            let handle = thread::spawn(move || {
                let mut batch_s = vec![FE::zero(); domain_size];
                let mut batch_m = vec![FE::zero(); domain_size];
                
                for (_local_idx, &client_x) in x_val_batch.iter().enumerate() {
                    let mut shifted_val_1_0 = vec![FE::zero(); domain_size];
                    let mut shifted_val_2_0 = vec![FE::zero(); domain_size];
                    
                    for i in 0..domain_size {
                        let idx = (i + client_x as usize) % domain_size;
                        shifted_val_1_0[i] = values1_0_batch[idx].clone();
                        shifted_val_2_0[i] = values2_0_batch[idx].clone();
                    }

                    let mut cumulative_s0 = FE::zero();
                    let mut cumulative_m0 = FE::zero();
                    
                    for i in 0..domain_size {
                        cumulative_s0.add(&shifted_val_1_0[i]);
                        batch_s[i].add(&cumulative_s0.clone());
                        cumulative_m0.add(&shifted_val_2_0[i]);
                        batch_m[i].add(&cumulative_m0.clone());
                    }
                }
                (batch_s, batch_m)
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let (batch_s, batch_m) = handle.join().unwrap();
            for i in 0..domain_size {
                all_client_s0[i].add(&batch_s[i]);
                all_client_m0[i].add(&batch_m[i]);
            }
        }
        
        (all_client_s0, all_client_m0)
    });

    let _ready_response = response_rx.recv().unwrap();
    let (all_client_s0, all_client_m0) = client_processing_handle.join().unwrap();
    client_processing_time = client_processing_start.elapsed();

    let online_start = Instant::now();
    let mut all_opened_values = Vec::new();
    let mut all_mac_shares_0 = Vec::new();

    // ROUND 1 - OPTIMIZED
    // println!("Round 1: Opening x2 values");
    let round1_compute_start = Instant::now();
    
    // let mut now = Local::now();
    // println!("r1 compute start: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));
    let x2_computation_handle = thread::spawn(move || {
        let mut x2_shares_0 = Vec::with_capacity(domain_size);
        for idx in 0..domain_size {
            let x2_0_fe = r2_0.clone() - all_client_s0[idx].clone();
            x2_shares_0.push(x2_0_fe);
        }
        x2_shares_0
    });

    let x2_shares_0 = x2_computation_handle.join().unwrap();
    round1_compute_time = round1_compute_start.elapsed();
    // now = Local::now();
    // println!("r1 compute end: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));
    // println!("🟦 PARTY0: Computed x2_shares_0[0] = {}", x2_shares_0[0].value());

    let round1_comm_start = Instant::now();
    // OPTIMIZED: Bulk serialization for Round 1
    let serialize_start = Instant::now();
    let round1_msg = Message::Round1X2Opening {
        x2_shares_bulk: bulk_fe_to_bytes(&x2_shares_0),  // OPTIMIZED
    };
    let round1_send_size = bincode::serialize(&round1_msg).unwrap().len();
    online_comm_size += round1_send_size;
    // println!("📤 Round 1 SEND: {} bytes", round1_send_size);
    // TEST CORRECTNESS
    // let x2_bulk_bytes = bulk_fe_to_bytes(&x2_shares_0);
    // println!("🟦 PARTY0: Serialized {} FEs into {} bytes", x2_shares_0.len(), x2_bulk_bytes.len());
    // println!("🟦 PARTY0: First 8 bytes of serialized data: {:?}", &x2_bulk_bytes[0..8.min(x2_bulk_bytes.len())]);
    // // Test immediate deserialization:
    // let test_deserialize = bulk_bytes_to_fe(&x2_bulk_bytes);
    // println!("🟦 PARTY0: Test deserialize x2_shares_0[0] = {}", test_deserialize[0].value());
    
    // println!("Round 1 serialization took: {:?}", serialize_start.elapsed());
    
    send_tx.send(SendCommand::Send(round1_msg)).unwrap();
    // now = Local::now();
    // println!("r1 send 1: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));
    
    receive_tx.send(ReceiveCommand::Receive).unwrap();
    // now = Local::now();
    // println!("r1 send 2: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));

    
    round1_comm_time = round1_comm_start.elapsed();
    let round1_response = response_rx.recv().unwrap();
    
    
    let Message::Round1X2Opening { x2_shares_bulk: x2_shares_1_bulk } = round1_response else {
        panic!("Expected Round1X2Opening");
    };

    let round1_process_start = Instant::now();
    // OPTIMIZED: Bulk deserialization for Round 1
    let deserialize_start = Instant::now();
    let x2_shares_1 = bulk_bytes_to_fe(&x2_shares_1_bulk);  // OPTIMIZED
    // println!("Round 1 deserialization took: {:?}", deserialize_start.elapsed());
    
    // now = Local::now();
        // println!("r1 recv: {}", now.format("%Y-%m-%d %H:%M:%S%.6f"));

    let col_values_clone = (col_sum_values1_0.clone(), col_sum_values2_0.clone());
    let alpha_r2_0_clone = alpha_r2_0.clone();
    let all_client_m0_clone = all_client_m0.clone();
    
    let x2_processing_handle = thread::spawn(move || {
        let mut all_col_shifted_values = Vec::new();
        let mut opened_values = Vec::new();
        let mut mac_shares = Vec::new();
        
        for (idx, x2_1_fe) in x2_shares_1.iter().enumerate() {
            let x2_fe = x2_shares_0[idx].clone() + x2_1_fe.clone();
            // println!(" The x2 {:?}", x2_fe.value());
            
            
            opened_values.push(fe_to_bytes(&x2_fe));
            let alpha_x2_0 = alpha_r2_0_clone.clone() - all_client_m0_clone[idx].clone();
            mac_shares.push(fe_to_bytes(&alpha_x2_0));
            
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

            // println!(" The x2_val at {} is {:?}", idx, x2_val);
            let mut col_sum_shifted_val_1_0 = vec![FE::zero(); updated_domain];
            let mut col_sum_shifted_val_2_0 = vec![FE::zero(); updated_domain];
            
            for i in 0..updated_domain {
                let shift_idx = (i + x2_val as usize) % updated_domain;
                col_sum_shifted_val_1_0[i] = col_values_clone.0[shift_idx].clone();
                col_sum_shifted_val_2_0[i] = col_values_clone.1[shift_idx].clone();
            }
            
            all_col_shifted_values.push((col_sum_shifted_val_1_0, col_sum_shifted_val_2_0));
        }
        
        (all_col_shifted_values, opened_values, mac_shares)
    });

    let (all_col_shifted_values, mut round1_opened, mut round1_mac) = x2_processing_handle.join().unwrap();
    all_opened_values.append(&mut round1_opened);
    all_mac_shares_0.append(&mut round1_mac);
    round1_process_time = round1_process_start.elapsed();

    let mut current_threshold = num_clients - 1;
    let mut second_highest_found = false;
    let mut second_highest_bid = 0;

    let tie_values1_0_clone = tie_values1_0.clone();
    let tie_values2_0_clone = tie_values2_0.clone();
    
    while current_threshold > 0 && !second_highest_found {
        // println!("Checking for threshold: {} bidders", current_threshold);
        
        // ROUND 2
        let round2_compute_start = Instant::now();
        let mut threshold_sum_0 = FE::zero();
        let mut threshold_mac_0 = FE::zero();
        
        for idx in 0..domain_size {
            let (ref col_1_0, ref col_2_0) = &all_col_shifted_values[idx];
            threshold_sum_0.add(&col_1_0[current_threshold]);
            threshold_mac_0.add(&col_2_0[current_threshold]);
        }
        
        let r3_shift_0 = r3_0.clone() - threshold_sum_0.clone();
        round2_compute_time += round2_compute_start.elapsed();
        
        let round2_comm_start = Instant::now();
        let round2_msg = Message::Round3R3Shift {
            r3_shift_share: fe_to_bytes(&r3_shift_0),
        };
        let round2_send_size = bincode::serialize(&round2_msg).unwrap().len();
        online_comm_size += round2_send_size;
        // println!("📤 Round 2 SEND: {} bytes", round2_send_size);
        
        send_tx.send(SendCommand::Send(round2_msg)).unwrap();
        receive_tx.send(ReceiveCommand::Receive).unwrap();
        
        let round2_response = response_rx.recv().unwrap();
        round2_comm_time += round2_comm_start.elapsed();
        
        let Message::Round3R3Shift { r3_shift_share: r3_shift_1_bytes } = round2_response else {
            panic!("Expected Round3R3Shift");
        };
        
        let r3_shift_1 = bytes_to_fe(&r3_shift_1_bytes);
        let r3_opened = r3_shift_0 + r3_shift_1;
        
        all_opened_values.push(fe_to_bytes(&r3_opened));
        let alpha_r3_threshold_0 = alpha_r3_0.clone() - threshold_mac_0.clone();
        all_mac_shares_0.push(fe_to_bytes(&alpha_r3_threshold_0));
        
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

        let tie_values1_0_for_thread = tie_values1_0_clone.clone();
        let tie_values2_0_for_thread = tie_values2_0_clone.clone();
        let tie_shifting_handle = thread::spawn(move || {
            let mut tie_shifted_val_1_0 = vec![FE::zero(); max_possible_sum];
            let mut tie_shifted_val_2_0 = vec![FE::zero(); max_possible_sum];
            
            for i in 0..max_possible_sum {
                let shift_idx = (i + r3_shift_val as usize) % max_possible_sum;
                tie_shifted_val_1_0[i] = tie_values1_0_for_thread[shift_idx].clone();
                tie_shifted_val_2_0[i] = tie_values2_0_for_thread[shift_idx].clone();
            }
            (tie_shifted_val_1_0, tie_shifted_val_2_0)
        });

        let (tie_shifted_val_1_0, tie_shifted_val_2_0) = tie_shifting_handle.join().unwrap();

        // ROUND 4
        let round4_comm_start = Instant::now();
        let exact_one_check_0 = tie_shifted_val_1_0[0].clone();
        
        let round4_msg = Message::Round4TieResult {
            tie_result_share: fe_to_bytes(&exact_one_check_0),
        };

        let round4_send_size = bincode::serialize(&round4_msg).unwrap().len();
        online_comm_size += round4_send_size;
        // println!("📤 Round 4 SEND: {} bytes", round4_send_size);
        
        send_tx.send(SendCommand::Send(round4_msg)).unwrap();
        receive_tx.send(ReceiveCommand::Receive).unwrap();
        
        let round4_response = response_rx.recv().unwrap();
        round4_comm_time += round4_comm_start.elapsed();
        
        let Message::Round4TieResult { tie_result_share: exact_one_check_1_bytes } = round4_response else {
            panic!("Expected Round4TieResult");
        };
        
        let exact_one_check_1 = bytes_to_fe(&exact_one_check_1_bytes);
        let exact_one_check = exact_one_check_0 + exact_one_check_1;
        
        all_opened_values.push(fe_to_bytes(&exact_one_check));
        all_mac_shares_0.push(fe_to_bytes(&tie_shifted_val_2_0[0]));
        
        second_highest_found = true;

        if exact_one_check.value() == 0 || true {
            // ROUND 5 - OPTIMIZED
            let round5_compute_start = Instant::now();
            let col_computation_handle = thread::spawn(move || {
                let mut col_ge_threshold_shares_0 = Vec::with_capacity(domain_size);
                let mut mac_accum_shares_0 = Vec::with_capacity(domain_size);

                for idx in 0..domain_size {
                    let (ref col_1_0, ref col_2_0) = &all_col_shifted_values[idx];
                    
                    let mut col_ge_threshold_0 = FE::zero();
                    let mut mac_accum_0 = FE::zero();
                    
                    for j in current_threshold..updated_domain {
                        col_ge_threshold_0.add(&col_1_0[j]);
                        mac_accum_0.add(&col_2_0[j]);
                    }
                    
                    col_ge_threshold_shares_0.push(col_ge_threshold_0);
                    mac_accum_shares_0.push(mac_accum_0);
                }
                (col_ge_threshold_shares_0, mac_accum_shares_0)
            });

            let (col_ge_threshold_shares_0, mac_accum_shares_0) = col_computation_handle.join().unwrap();
            round5_compute_time = round5_compute_start.elapsed();

            let round5_comm_start = Instant::now();
            // OPTIMIZED: Bulk serialization for Round 5
            let serialize_start = Instant::now();
            let round5_msg = Message::Round5SecondHighest {
                found_second_highest: true,
                second_highest_bid: 0,
                col_ge_threshold_shares_bulk: bulk_fe_to_bytes(&col_ge_threshold_shares_0), // OPTIMIZED
            };
            println!("Round 5 serialization took: {:?}", serialize_start.elapsed());

            let round5_send_size = bincode::serialize(&round5_msg).unwrap().len();
            online_comm_size += round5_send_size;
            println!("📤 Round 5 SEND: {} bytes", round5_send_size);
            println!("Round 5 serialization took: {:?}", serialize_start.elapsed());
            
            send_tx.send(SendCommand::Send(round5_msg)).unwrap();
            receive_tx.send(ReceiveCommand::Receive).unwrap();

            let round5_response = response_rx.recv().unwrap();
            round5_comm_time = round5_comm_start.elapsed();
            
            let Message::Round5SecondHighest { col_ge_threshold_shares_bulk: col_ge_threshold_shares_1_bulk, .. } = round5_response else {
                panic!("Expected Round5SecondHighest");
            };

            // OPTIMIZED: Bulk deserialization for Round 5
            let deserialize_start = Instant::now();
            let col_ge_threshold_shares_1 = bulk_bytes_to_fe(&col_ge_threshold_shares_1_bulk); // OPTIMIZED
            // println!("Round 5 deserialization took: {:?}", deserialize_start.elapsed());

            for idx in 0..domain_size {
                let col_ge_threshold = col_ge_threshold_shares_0[idx].clone() + col_ge_threshold_shares_1[idx].clone();
                
                if col_ge_threshold.value() >= 1 {
                    second_highest_bid = idx;
                    all_opened_values.push(fe_to_bytes(&col_ge_threshold));
                    all_mac_shares_0.push(fe_to_bytes(&mac_accum_shares_0[idx]));
                    break;
                }
            }

            // ROUND 6 - OPTIMIZED
            let round6_compute_start = Instant::now();
            let winner_computation_handle = thread::spawn(move || {
                let mut temp_sum_shares_0 = Vec::with_capacity(num_clients);
                let mut temp_sum_mac_shares_0 = Vec::with_capacity(num_clients);

                for bidder in 0..num_clients {
                    let mut temp_sum_0 = FE::zero();
                    let mut temp_sum_mac_0 = FE::zero();
                    
                    for index in 0..=second_highest_bid {
                        let sh_index = (index + x_val[bidder] as usize) % domain_size;
                        temp_sum_0.add(&values1_0[sh_index]);
                        temp_sum_mac_0.add(&values2_0[sh_index]);
                    }
                    
                    temp_sum_shares_0.push(temp_sum_0);
                    temp_sum_mac_shares_0.push(temp_sum_mac_0);
                }
                (temp_sum_shares_0, temp_sum_mac_shares_0)
            });

            let (temp_sum_shares_0, temp_sum_mac_shares_0) = winner_computation_handle.join().unwrap();
            round6_compute_time = round6_compute_start.elapsed();

            let round6_comm_start = Instant::now();
            // OPTIMIZED: Bulk serialization for Round 6
            let serialize_start = Instant::now();
            let round6_msg = Message::Round6Winner {
                temp_sum_shares_bulk: bulk_fe_to_bytes(&temp_sum_shares_0), // OPTIMIZED
            };
        
            let round6_send_size = bincode::serialize(&round6_msg).unwrap().len();
            online_comm_size += round6_send_size;
            // println!("📤 Round 6 SEND: {} bytes", round6_send_size);
            // println!("Round 6 serialization took: {:?}", serialize_start.elapsed());
            
            send_tx.send(SendCommand::Send(round6_msg)).unwrap();
            receive_tx.send(ReceiveCommand::Receive).unwrap();

            let round6_response = response_rx.recv().unwrap();
            round6_comm_time = round6_comm_start.elapsed();
            
            let Message::Round6Winner { temp_sum_shares_bulk: temp_sum_shares_1_bulk } = round6_response else {
                panic!("Expected Round6Winner");
            };

            // OPTIMIZED: Bulk deserialization for Round 6
            let deserialize_start = Instant::now();
            let temp_sum_shares_1 = bulk_bytes_to_fe(&temp_sum_shares_1_bulk); // OPTIMIZED
            // println!("Round 6 deserialization took: {:?}", deserialize_start.elapsed());

            let mut highest_bidder = 0;
            for bidder in 0..num_clients {
                let temp_sum_total = temp_sum_shares_0[bidder].clone() + temp_sum_shares_1[bidder].clone();
                
                all_opened_values.push(fe_to_bytes(&temp_sum_total));
                all_mac_shares_0.push(fe_to_bytes(&temp_sum_mac_shares_0[bidder]));
                
                if temp_sum_total.value() == 0 {
                    highest_bidder = bidder;
                }
            }

            // MAC VERIFICATION - OPTIMIZED
            let mac_start = Instant::now();
            // OPTIMIZED: Bulk serialization for MAC verification
            let serialize_start = Instant::now();
            let final_msg = Message::FinalMacVerification {
                alpha_share: fe_to_bytes(&alpha_val_0),
                all_opened_values_bulk: all_opened_values.iter().flat_map(|v| v.iter()).cloned().collect(), // OPTIMIZED
                all_mac_shares_bulk: all_mac_shares_0.iter().flat_map(|v| v.iter()).cloned().collect(),    // OPTIMIZED
            };

            let mac_send_size = bincode::serialize(&final_msg).unwrap().len();
            online_comm_size += mac_send_size;
            // println!("📤 MAC Verification SEND: {} bytes", mac_send_size);
            // println!("MAC verification serialization took: {:?}", serialize_start.elapsed());

            send_tx.send(SendCommand::Send(final_msg)).unwrap();
            receive_tx.send(ReceiveCommand::Receive).unwrap();

            let final_response = response_rx.recv().unwrap();
            let Message::FinalMacVerification { alpha_share: alpha_1_bytes, all_mac_shares_bulk: mac_shares_1_bulk, .. } = final_response else {
                panic!("Expected FinalMacVerification");
            };

            let alpha_1 = bytes_to_fe(&alpha_1_bytes);

            // OPTIMIZED: Bulk deserialization for MAC verification
            let deserialize_start = Instant::now();
            let mac_shares_1_vec: Vec<Vec<u8>> = mac_shares_1_bulk.chunks_exact(8).map(|chunk| chunk.to_vec()).collect(); // OPTIMIZED
            // println!("MAC verification deserialization took: {:?}", deserialize_start.elapsed());

            let opened_values_for_verification = all_opened_values.clone();
            let mac_shares_0_for_verification = all_mac_shares_0.clone();
            let num_checks = opened_values_for_verification.len();

            let mac_verification_handle = thread::spawn(move || {
                let alpha_reconstructed = alpha_val_0 + alpha_1;
                let mut all_passed = true;
                
                for i in 0..opened_values_for_verification.len() {
                    let opened_value = bytes_to_fe(&opened_values_for_verification[i]);
                    let mac_0 = bytes_to_fe(&mac_shares_0_for_verification[i]);
                    let mac_1 = bytes_to_fe(&mac_shares_1_vec[i]);
                    
                    let mut z_0 = opened_value.clone();
                    z_0.mul(&alpha_val_0);
                    z_0.sub(&mac_0);
                    
                    let mut z_1 = opened_value.clone();
                    z_1.mul(&alpha_1);
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
                println!("All {} MAC checks passed on Party 0!", num_checks);
            } else {
                println!("Some MAC checks failed!");
            }

            println!("Alpha reconstructed: {}", alpha_reconstructed.value());
            println!("Second highest bid: {}", second_highest_bid);
            println!("Highest bidder: {}", highest_bidder);

            send_tx.send(SendCommand::Send(Message::Result(second_highest_bid as u64, highest_bidder as u64))).unwrap();
            break;
        } else {
            current_threshold -= 1;
            println!("Checking next threshold: {}", current_threshold);
        }
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

    // NEW: Online communication summary
    println!("\n=== ONLINE COMMUNICATION BREAKDOWN ===");
    println!("📊 Total online communication: {} bytes ({:.2} KB, {:.2} MB)", 
         online_comm_size, 
         online_comm_size as f64 / 1024.0,
         online_comm_size as f64 / (1024.0 * 1024.0));
    
    // Parseable summary for scripts
    println!("\n=== BENCHMARK SUMMARY ===");
    println!("PREPROCESS_TIME_MS: {:.3}", preprocessing_time.as_secs_f64() * 1000.0);
    println!("ONLINE_TIME_MS: {:.3}", online_time.as_secs_f64() * 1000.0);
    println!("PREPROCESS_COMM_BYTES: {}", preprocessing_comm_size);
    println!("ONLINE_COMM_BYTES: {}", online_comm_size);
}
