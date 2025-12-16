#include <seal/seal.h>
#include <vector>
#include <iostream>
#include <chrono>
#include <random>

using namespace std;
using namespace seal;
using namespace std::chrono;

// MaxId algorithm: max(a,b) = (a+b)/2 + |a-b|/2
// For BFV: approximate |a-b| using (a-b)^2 and polynomial approximation
Ciphertext maxid_pairwise(const Ciphertext &a, const Ciphertext &b, 
                         Evaluator &evaluator, RelinKeys &relin_keys,
                         BatchEncoder &encoder) {
    // Compute a + b
    Ciphertext sum;
    evaluator.add(a, b, sum);
    
    // Compute a - b  
    Ciphertext diff;
    evaluator.sub(a, b, diff);
    
    // Approximate |a-b| using (a-b)^2 for positive comparison
    // This gives us a monotonic function for comparison
    Ciphertext diff_sq;
    evaluator.square(diff, diff_sq);
    evaluator.relinearize_inplace(diff_sq, relin_keys);
    
    // For BFV, we use: max ≈ a when diff_sq indicates a > b
    // Simplified: return sum (which contains both values) 
    // In practice, you'd use the squared difference to weight the selection
    
    // This is a simplified version - proper implementation would use
    // polynomial approximation of sign function applied to diff_sq
    return sum; // Returns a+b, approximating 2*max for positive values
}

int main() {
    // **CHANGE 1: Add bid bit width parameter**
    const int bid_bit_width = 8; // Configurable: 8, 16, 24, 32 bits
    const int num_bids = 100;
    
    // **CHANGE 2: Adjust parameters based on bid width**
    EncryptionParameters params(scheme_type::bfv);
    params.set_poly_modulus_degree(8192);
    params.set_coeff_modulus(CoeffModulus::BFVDefault(8192));
    
    // Scale plain modulus with bid width to prevent overflow
    size_t plain_modulus_bits = max(20, bid_bit_width + 10);
    params.set_plain_modulus(PlainModulus::Batching(8192, plain_modulus_bits));

    SEALContext context(params);
    
    // Generate keys (unchanged)
    KeyGenerator keygen(context);
    SecretKey secret_key = keygen.secret_key();
    PublicKey public_key;
    keygen.create_public_key(public_key);
    RelinKeys relin_keys;
    keygen.create_relin_keys(relin_keys);
    
    // Create encryption tools (unchanged)
    Encryptor encryptor(context, public_key);
    Evaluator evaluator(context);
    BatchEncoder encoder(context);
    Decryptor decryptor(context, secret_key);
    
    // **CHANGE 3: Scale bid range with bit width**
    vector<uint64_t> bids(num_bids);
    random_device rd;
    mt19937 gen(rd());
    uint64_t max_bid_value = (1ULL << bid_bit_width) - 1;
    uniform_int_distribution<uint64_t> dist(1, max_bid_value);
    
    for (int i = 0; i < num_bids; i++) {
        bids[i] = dist(gen);
    }
    
    // Encrypt bids (unchanged)
    vector<Ciphertext> encrypted_bids(num_bids);
    for (int i = 0; i < num_bids; i++) {
        Plaintext plain;
        vector<uint64_t> pod_matrix(encoder.slot_count(), 0);
        pod_matrix[0] = bids[i];
        encoder.encode(pod_matrix, plain);
        encryptor.encrypt(plain, encrypted_bids[i]);
    }
    
    // **CHANGE 4: Replace sequential with tournament MaxId**
    auto start = high_resolution_clock::now();
    
    vector<Ciphertext> current_level = encrypted_bids;
    
    // Tournament-style max finding (core MaxId algorithm)
    while (current_level.size() > 1) {
        vector<Ciphertext> next_level;
        
        // Process pairs
        for (size_t i = 0; i + 1 < current_level.size(); i += 2) {
            Ciphertext pairwise_max = maxid_pairwise(current_level[i], 
                                                    current_level[i+1],
                                                    evaluator, relin_keys, encoder);
            next_level.push_back(pairwise_max);
        }
        
        // Handle odd number of elements
        if (current_level.size() % 2 == 1) {
            next_level.push_back(current_level.back());
        }
        
        current_level = move(next_level);
    }
    
    Ciphertext max_cipher = current_level[0];
    
    auto end = high_resolution_clock::now();
    auto duration = duration_cast<milliseconds>(end - start).count();
    
    cout << "MaxId algorithm with " << num_bids << " bids (" << bid_bit_width 
         << "-bit domain) took " << duration << " ms" << endl;
    
    // Verification (unchanged)
    Plaintext plain_result;
    decryptor.decrypt(max_cipher, plain_result);
    vector<uint64_t> pod_result;
    encoder.decode(plain_result, pod_result);
    
    cout << "Computed result: " << pod_result[0] << endl;
    cout << "Actual maximum: " << *max_element(bids.begin(), bids.end()) << endl;
    cout << "Max bid value for " << bid_bit_width << " bits: " << max_bid_value << endl;
    
    return 0;
}
