use std::fs::File;
use std::io::Write;
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
    
    let alpha_val_recon = alpha_val_0.clone() + alpha_val_1.clone();
    let mac_check = alpha_val_recon * z1 - z_star;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = std::env::args().nth(1).unwrap_or_else(|| "auction_data.bin".to_string());
    
    let num_clients = 100;
    let domain_size = 1000;

    // Preprocessing phase (not timed)
    let alpha_val = FE::random();
    
    let ((key1_0, key1_1), (key2_0, key2_1), r, (r_0, r_1), (alpha_val_0, alpha_val_1)) = 
        preprocess_mac(domain_size, &alpha_val);
    
    let values1_0 = eval_all(&key1_0, domain_size);
    let values1_1 = eval_all(&key1_1, domain_size);
    let values2_0 = eval_all(&key2_0, domain_size);
    let values2_1 = eval_all(&key2_1, domain_size);

    let updated_domain = num_clients + 1;
    let ((col_key1_0, col_key1_1), (col_key2_0, col_key2_1), r2, (r2_0, r2_1), (alpha_val2_0, alpha_val2_1)) = 
        preprocess_mac(updated_domain, &alpha_val);
    
    let alpha_r2 = alpha_val.clone() * r2.clone();
    let (alpha_r2_0, alpha_r2_1) = generate_alpha_shares(&alpha_r2);

    let col_sum_values1_0 = eval_all(&col_key1_0, updated_domain);
    let col_sum_values1_1 = eval_all(&col_key1_1, updated_domain);
    let col_sum_values2_0 = eval_all(&col_key2_0, updated_domain);
    let col_sum_values2_1 = eval_all(&col_key2_1, updated_domain);

    let max_possible_sum = domain_size;
    let ((tie_key1_0, tie_key1_1), (tie_key2_0, tie_key2_1), r3, (r3_0, r3_1), (alpha_val3_0, alpha_val3_1)) = 
        preprocess_mac(max_possible_sum, &alpha_val);
    
    let alpha_r3 = alpha_val.clone() * r3.clone();
    let (alpha_r3_0, alpha_r3_1) = generate_alpha_shares(&alpha_r3);

    let tie_values1_0 = eval_all(&tie_key1_0, max_possible_sum);
    let tie_values1_1 = eval_all(&tie_key1_1, max_possible_sum);
    let tie_values2_0 = eval_all(&tie_key2_0, max_possible_sum);
    let tie_values2_1 = eval_all(&tie_key2_1, max_possible_sum);
    
    mal_preprocess_check(&col_sum_values1_0, &col_sum_values1_1, &col_sum_values2_0, &col_sum_values2_1, 
        updated_domain, &r2, &alpha_val, &r2_0, &r2_1, &alpha_val2_0, &alpha_val2_1);
    mal_preprocess_check(&values1_0, &values1_1, &values2_0, &values2_1, domain_size, &r, &alpha_val, &r_0, &r_1, &alpha_val_0, &alpha_val_1);
    mal_preprocess_check(&tie_values1_0, &tie_values1_1, &tie_values2_0, &tie_values2_1, 
        max_possible_sum, &r3, &alpha_val, &r3_0, &r3_1, &alpha_val3_0, &alpha_val3_1);

    let mut x_val = vec![0; num_clients];

    for client in 0..num_clients {
        let (one_hot, lsb_hot_index) = generate_one_hot_conventional(domain_size);
        let a_index = domain_size - 1 - lsb_hot_index;
        let a_val = FE::from(a_index as u32);

        let (a_0, a_1) = generate_alpha_shares(&a_val);

        let x_share0: u64 = (r_0.value() + domain_size as u64 - a_0.value()) % (domain_size as u64);
        let x_share1: u64 = (r_1.value() + domain_size as u64 - a_1.value()) % (domain_size as u64);

        x_val[client] = (x_share0 + x_share1) % (domain_size as u64);
    }

    // Package all data
    let auction_data = AuctionData {
        num_clients,
        domain_size,
        updated_domain,
        max_possible_sum,
        values1_0,
        values1_1,
        values2_0,
        values2_1,
        r,
        r_0,
        r_1,
        alpha_val_0,
        alpha_val_1,
        col_sum_values1_0,
        col_sum_values1_1,
        col_sum_values2_0,
        col_sum_values2_1,
        r2,
        r2_0,
        r2_1,
        alpha_val2_0,
        alpha_val2_1,
        alpha_r2_0,
        alpha_r2_1,
        tie_values1_0,
        tie_values1_1,
        tie_values2_0,
        tie_values2_1,
        r3,
        r3_0,
        r3_1,
        alpha_val3_0,
        alpha_val3_1,
        alpha_r3_0,
        alpha_r3_1,
        alpha_val,
        x_val,
    };

    // Save to file
    let encoded = bincode::serialize(&auction_data)?;
    let mut file = File::create(&filename)?;
    file.write_all(&encoded)?;
    
    println!("Preprocessing complete! Saved to {}", filename);
    println!("Data size: {:.2} MB", encoded.len() as f64 / 1024.0 / 1024.0);
    
    Ok(())
}
