// common.rs - FIXED with correct FE API
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use crate::fastfield::FE;
use crate::prg::FromRng;
use crate::Group; // For zero() method
use std::ops::{Add, Mul}; // For add() and mul() methods

// FIXED: Use correct FE constructors and methods
pub fn bulk_fe_to_bytes(fes: &[FE]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(fes.len() * 8);
    for fe in fes {
        // Store the full u64 value
        bytes.extend_from_slice(&fe.value().to_le_bytes());
    }
    bytes
}

pub fn bulk_bytes_to_fe(bytes: &[u8]) -> Vec<FE> {
    assert_eq!(bytes.len() % 8, 0);
    let mut fes = Vec::with_capacity(bytes.len() / 8);
    for chunk in bytes.chunks_exact(8) {
        let mut array = [0u8; 8];
        array.copy_from_slice(chunk);
        let value = u64::from_le_bytes(array);
        
        // FIXED: Use FE::new which accepts u64 directly
        fes.push(FE::new(value));
    }
    fes
}

// FIXED: Single FE conversion functions
pub fn fe_to_bytes(fe: &FE) -> Vec<u8> {
    fe.value().to_le_bytes().to_vec()
}

pub fn bytes_to_fe(bytes: &[u8]) -> FE {
    let mut array = [0u8; 8];
    array[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
    let value = u64::from_le_bytes(array);
    
    // FIXED: Use FE::new which accepts u64 directly
    FE::new(value)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    PreprocessingData {
        alpha_share: Vec<u8>,
        r_share: Vec<u8>,
        r2_share: Vec<u8>,
        r3_share: Vec<u8>,
        values1_shares_bulk: Vec<u8>,
        values2_shares_bulk: Vec<u8>,
        col_values1_shares_bulk: Vec<u8>,
        col_values2_shares_bulk: Vec<u8>,
        tie_values1_shares_bulk: Vec<u8>,
        tie_values2_shares_bulk: Vec<u8>,
        alpha_r2_share: Vec<u8>,
        alpha_r3_share: Vec<u8>,
        x_values: Vec<u64>,
    },
    
    Round1X2Opening {
        x2_shares_bulk: Vec<u8>,
    },
    
    Round3R3Shift {
        r3_shift_share: Vec<u8>,
    },
    
    Round4TieResult {
        tie_result_share: Vec<u8>,
    },
    
    Round5SecondHighest {
        found_second_highest: bool,
        second_highest_bid: usize,
        col_ge_threshold_shares_bulk: Vec<u8>,
    },
    
    Round6Winner {
        temp_sum_shares_bulk: Vec<u8>,
    },
    
    FinalMacVerification {
        alpha_share: Vec<u8>,
        all_opened_values_bulk: Vec<u8>,
        all_mac_shares_bulk: Vec<u8>,
    },
    
    Ready,
    Result(u64, u64),
}

pub fn send_message(stream: &mut TcpStream, message: &Message) -> std::io::Result<()> {
    let serialized = bincode::serialize(message).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })?;
    let len = serialized.len() as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(&serialized)?;
    stream.flush()?;
    Ok(())
}

pub fn receive_message(stream: &mut TcpStream) -> std::io::Result<Message> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    
    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer)?;
    
    bincode::deserialize(&buffer).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    })
}

pub fn generate_alpha_shares<T: FromRng + Clone + Group>(alpha_val: &T) -> (T, T) {
    let mut share1 = T::zero();
    share1.randomize();
    let mut share2 = alpha_val.clone();
    share2.sub(&share1);
    (share1, share2)
}
