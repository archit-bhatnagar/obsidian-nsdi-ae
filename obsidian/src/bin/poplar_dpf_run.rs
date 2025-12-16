// src/bin/cumulative_dpf.rs
use counttree::dpf::*;
use counttree::*;
use counttree::fastfield::FE;
use counttree::sketch::*; // Import the sketch module
use counttree::mpc;
use rand::Rng;

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

/// Evaluates the SketchDPFKey cumulatively over the domain.
/// At every index (in natural LSB‑first order), we call the key's eval() function
/// and accumulate the outputs.
fn evaluate_cumulative_sketch(
    key: &SketchDPFKey<FE, FE>,
    length: usize,
) -> Vec<FE> {
    let mut result = Vec::with_capacity(length);
    let mut cumulative = FE::zero();
    for i in 0..length {
        let bits = u32_to_bits(6, i as u32); // For a domain of 64 (i.e. 6 bits)
        let val = key.eval(&bits);
        cumulative.add(&val);
        result.push(cumulative.clone());
    }
    result
}

// /// Part A: Generates the full sketch (triplet) using the two keys and a challenge vector.
// /// Returns the combined SketchOutput containing r_x, r2_x, and r_kx.
// fn generate_sketch_triplet(
//     keys: &[SketchDPFKey<FE, FE>],
//     challenge: &[(FE, FE)]
// ) -> SketchOutput<FE> {
//     let mut sketch_out = keys[0].sketch_at(challenge, &mut rand::thread_rng());
//     let sketch_out_1 = keys[1].sketch_at(challenge, &mut rand::thread_rng());
//     sketch_out.add(&sketch_out_1);
//     sketch_out.reduce();
//     sketch_out
// }

// /// Part B: Verifies the given triplet and prints the result.
// /// The verification checks two conditions:
// ///   1) (mac_key_total * r_x + mac_key2_total) == r_kx.
// ///   2) r_x^2 - r2_x == 0.
// fn verify_sketch_triplet(
//     keys: &[SketchDPFKey<FE, FE>],
//     triplet: &SketchOutput<FE>
// ) -> bool {
//     // Combine MAC keys.
//     let mut mac = FE::zero();
//     mac.add(&keys[0].mac_key);
//     mac.add(&keys[1].mac_key);
    
//     let mut mac2 = FE::zero();
//     mac2.add(&keys[0].mac_key2);
//     mac2.add(&keys[1].mac_key2);
    
//     // Verify MAC relation: mac * r_x + mac2 should equal r_kx.
//     let mut expected_r_kx = triplet.r_x.clone();
//     expected_r_kx.mul(&mac);
//     expected_r_kx.add(&mac2);
    
//     let mac_ok = expected_r_kx == triplet.r_kx;
    
//     // Verify internal consistency: r_x^2 - r2_x should be zero.
//     let mut square_check = triplet.r_x.clone();
//     square_check.mul(&triplet.r_x);
//     square_check.sub(&triplet.r2_x);
//     let internal_ok = square_check == FE::zero();
    
//     if !mac_ok {
//         println!("Sketch MAC verification FAILED: expected {:?} but got {:?}", expected_r_kx, triplet.r_kx);
//     }
//     if !internal_ok {
//         println!("Sketch internal consistency FAILED: r_x^2 - r2_x != 0");
//     }
    
//     mac_ok && internal_ok
// }

fn main() {
    let num_clients = 10;
    let domain_size = 64; // Domain size of 64 positions.
    println!(
        "Generating {} one‑hot encoded strings (conventional MSB‑first) of length {}\n",
        num_clients, domain_size
    );
    
    // We will store the sketch keys for each client.
    // Each call to SketchDPFKey::gen returns a pair (an array of 2 keys), one for each server.
    let mut all_sketch_keys: Vec<[SketchDPFKey<FE, FE>; 2]> = Vec::with_capacity(num_clients);
    let mut original_one_hot_vectors: Vec<Vec<bool>> = Vec::with_capacity(num_clients);
    let mut original_random_numbers: Vec<usize> = Vec::with_capacity(num_clients);
    
    // For each client, generate a one‑hot input and the corresponding sketch keys.
    for _client in 0..num_clients {
        let (one_hot, lsb_hot_index) = generate_one_hot_conventional(domain_size);
        original_one_hot_vectors.push(one_hot);
        original_random_numbers.push(lsb_hot_index);
        
        // For a domain of 64 positions, we require 6 bits.
        // The sketch generation function expects an alpha bit string; here we use the
        // LSB‑first hot index directly to produce it.
        let alpha = u32_to_bits(6, lsb_hot_index as u32);
        // For intermediate levels we need a vector of length (alpha.len() - 1), here 5 elements.
        let betas = vec![FE::one(); alpha.len() - 1];
        let beta_last = FE::one();
        let keys = SketchDPFKey::gen(&alpha, &betas, &beta_last);
        all_sketch_keys.push(keys);
    }
    
    // Print the original one‑hot inputs.
    // These are stored in conventional order (MSB‑first) for human readability.
    println!("Original one‑hot inputs (MSB‑first):");
    for (client, vec_bool) in original_one_hot_vectors.iter().enumerate() {
        let bit_str: String = vec_bool.iter().map(|&b| if b { '1' } else { '0' }).collect();
        println!(
            "Client {}: Random hot index (LSB‑first): {}  One‑hot: {}",
            client, original_random_numbers[client], bit_str
        );
    }
    
    // Part 1: DPF Cumulative Evaluation using the underlying sketch keys.
    println!("\nDPF Cumulative Results (Conventional MSB‑first):");
    for client in 0..num_clients {
        let cum0 = evaluate_cumulative_sketch(&all_sketch_keys[client][0], domain_size);
        let cum1 = evaluate_cumulative_sketch(&all_sketch_keys[client][1], domain_size);
        
        let mut cumulative_bits = Vec::with_capacity(domain_size);
        let base = FE::zero().value(); // The internal representation of "zero"
        for i in 0..domain_size {
            let mut combined = cum0[i].clone();
            combined.add(&cum1[i]);
            let logical_val = if combined.value() >= base {
                combined.value() - base
            } else {
                0
            };
            cumulative_bits.push(if logical_val > 0 { '1' } else { '0' });
        }
        // Reverse so that the printed string is in conventional MSB‑first order.
        cumulative_bits.reverse();
        let cumulative_str: String = cumulative_bits.into_iter().collect();
        println!("Client {}: {}", client, cumulative_str);
    }

    

    // ----- Part 2: Sketch Verification Process (MPC Simulation) -----
    //
    // For each client we now generate the aggregated sketch values required for MPC verification.
    // These are:
    //   r_x  (the final cumulative value),
    //   r2_x = r_x^2,
    //   r_kx = (mac_key * r_x) + mac_key2 (using combined MAC keys),
    // along with random masks.
    //
    // The MPC machinery then simulates subtracting precomputed triple shares and using them to compute
    // correlation shares and output shares. The final MPC check verifies that the sum of the two servers'
    // output shares equals zero.
    for client in 0..num_clients {
        // Recompute the aggregated final cumulative value (r_x) for this client.
        let cum0 = evaluate_cumulative_sketch(&all_sketch_keys[client][0], domain_size);
        let cum1 = evaluate_cumulative_sketch(&all_sketch_keys[client][1], domain_size);
        let mut agg_r_x = cum0[domain_size - 1].clone();
        agg_r_x.add(&cum1[domain_size - 1]);
        
        // Compute r2_x as the square of r_x.
        let mut agg_r2_x = agg_r_x.clone();
        agg_r2_x.mul(&agg_r_x);
        
        // Combine MAC keys from both key shares.
        let mut agg_mac = all_sketch_keys[client][0].mac_key.clone();
        agg_mac.add(&all_sketch_keys[client][1].mac_key);
        let mut agg_mac2 = all_sketch_keys[client][0].mac_key2.clone();
        agg_mac2.add(&all_sketch_keys[client][1].mac_key2);
        
        // Compute r_kx = (agg_mac * r_x) + agg_mac2.
        let mut agg_r_kx = agg_r_x.clone();
        agg_r_kx.mul(&agg_mac);
        agg_r_kx.add(&agg_mac2);
        
        // Generate random masks following the sketch process.
        let rand1 = FE::random();
        let rand2 = FE::random();
        let rand3 = FE::random();
        
        // Build the aggregated SketchOutput.
        let sketch_output = SketchOutput {
            r_x: agg_r_x.clone(),
            r2_x: agg_r2_x.clone(),
            r_kx: agg_r_kx.clone(),
            rand1,
            rand2,
            rand3,
        };
        
        println!("\n--- MPC Verification for Client {} ---", client);
        println!("Aggregated Sketch:");
        println!("  r_x:  {:?}", sketch_output.r_x);
        println!("  r2_x: {:?}", sketch_output.r2_x);
        println!("  r_kx: {:?}", sketch_output.r_kx);
        
        // Prepare MPC inputs using the stored triple shares and MAC values.
        let triples = vec![
            all_sketch_keys[client][0].triples.clone(),
            all_sketch_keys[client][1].triples.clone(),
        ];
        // --- Fix: ensure each triple vector has at least 2 elements ---
        let triples: Vec<_> = triples
            .into_iter()
            .map(|mut t| {
                if t.len() < 2 {
                    t.push(t[0].clone());
                }
                t
            })
            .collect();
            
        let mac_keys = vec![
            all_sketch_keys[client][0].mac_key.clone(),
            all_sketch_keys[client][1].mac_key.clone(),
        ];
        let mac_keys2 = vec![
            all_sketch_keys[client][0].mac_key2.clone(),
            all_sketch_keys[client][1].mac_key2.clone(),
        ];
        
        // Simulate two servers computing the MPC values.
        let state_server0 = mpc::ManyMulState::new(
            false,
            &triples,
            &mac_keys,
            &mac_keys2,
            &[sketch_output.clone()],
            0,
        );
        let state_server1 = mpc::ManyMulState::new(
            true,
            &triples,
            &mac_keys,
            &mac_keys2,
            &[sketch_output.clone()],
            0,
        );
        
        // Each server computes its own correlation shares.
        let cor_shares0 = state_server0.cor_shares();
        let cor_shares1 = state_server1.cor_shares();
        println!("MPC Correlation Shares (Server 0): {:?}", cor_shares0);
        println!("MPC Correlation Shares (Server 1): {:?}", cor_shares1);
        
        // Combine the correlation shares from both servers.
        let many_cor = mpc::ManyMulState::cors(&cor_shares0, &cor_shares1);
        // Compute output shares from each server.
        let out_shares0 = state_server0.out_shares(&many_cor);
        let out_shares1 = state_server1.out_shares(&many_cor);
        println!("MPC Out Shares (Server 0): {:?}", out_shares0);
        println!("MPC Out Shares (Server 1): {:?}", out_shares1);
        
        // Final verification: the sum of the two servers' output shares should equal zero.
        let verification = mpc::ManyMulState::verify(&out_shares0, &out_shares1);
        println!("Final MPC Verification (should be all true): {:?}", verification);
    }
}