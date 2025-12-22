#include <seal/seal.h>
#include <vector>
#include <iostream>
#include <chrono>
#include <random>
#include <algorithm>
#include <cmath>
#include <memory>

using namespace std;
using namespace seal;
using namespace std::chrono;

class MaxIdBFV {
private:
    SEALContext context;
    KeyGenerator keygen;
    SecretKey secret_key;
    PublicKey public_key;
    RelinKeys relin_keys;
    unique_ptr<Encryptor> encryptor;
    Evaluator evaluator;
    BatchEncoder encoder;
    unique_ptr<Decryptor> decryptor;
    int bit_width;
    uint64_t max_bid_value;
    uint64_t plain_modulus_value;

public:
    MaxIdBFV(int bit_width) : bit_width(bit_width), 
                              max_bid_value((1ULL << bit_width) - 1),
                              context(setup_parameters(bit_width)),
                              keygen(context),
                              secret_key(keygen.secret_key()),
                              evaluator(context),
                              encoder(context) {
        
        keygen.create_public_key(public_key);
        keygen.create_relin_keys(relin_keys);
        
        encryptor = make_unique<Encryptor>(context, public_key);
        decryptor = make_unique<Decryptor>(context, secret_key);
        
        plain_modulus_value = context.first_context_data()->parms().plain_modulus().value();
    }

private:
    EncryptionParameters setup_parameters(int bits) {
        EncryptionParameters params(scheme_type::bfv);
        
        size_t poly_degree = (bits <= 8) ? 8192 : 
                            (bits <= 16) ? 16384 : 32768;
        params.set_poly_modulus_degree(poly_degree);
        params.set_coeff_modulus(CoeffModulus::BFVDefault(poly_degree));
        
        size_t plain_bits = max(20, bits + 10);
        params.set_plain_modulus(PlainModulus::Batching(poly_degree, plain_bits));
        
        return params;
    }

    Ciphertext pairwise_max(const Ciphertext& a, const Ciphertext& b) {
        Ciphertext sum;
        evaluator.add(a, b, sum);
        
        Ciphertext diff;
        evaluator.sub(a, b, diff);
        
        Ciphertext abs_diff;
        evaluator.square(diff, abs_diff);
        evaluator.relinearize_inplace(abs_diff, relin_keys);
        
        Ciphertext result;
        evaluator.add(sum, abs_diff, result);
        
        uint64_t inv_2 = modular_inverse(2, plain_modulus_value);
        Plaintext inv_2_plain;
        vector<uint64_t> inv_2_vec(encoder.slot_count(), inv_2);
        encoder.encode(inv_2_vec, inv_2_plain);
        evaluator.multiply_plain_inplace(result, inv_2_plain);
        
        return result;
    }
    
    uint64_t modular_inverse(uint64_t a, uint64_t mod) {
        int64_t m0 = mod, x0 = 0, x1 = 1;
        if (mod == 1) return 0;
        while (a > 1) {
            int64_t q = a / mod;
            int64_t t = mod;
            mod = a % mod;
            a = t;
            t = x0;
            x0 = x1 - q * x0;
            x1 = t;
        }
        if (x1 < 0) x1 += m0;
        return x1;
    }

public:
    Ciphertext encrypt_bid(uint64_t bid) {
        if (bid > max_bid_value) {
            throw invalid_argument("Bid exceeds maximum value");
        }
        
        Plaintext plain;
        vector<uint64_t> vec(encoder.slot_count(), 0);
        vec[0] = bid;
        encoder.encode(vec, plain);
        
        Ciphertext encrypted;
        encryptor->encrypt(plain, encrypted);
        return encrypted;
    }

    uint64_t decrypt_bid(const Ciphertext& encrypted) {
        Plaintext plain;
        decryptor->decrypt(encrypted, plain);
        vector<uint64_t> vec;
        encoder.decode(plain, vec);
        return vec[0];
    }

    // Calculate ciphertext size in bytes
    size_t get_ciphertext_size(const Ciphertext& ct) {
        auto context_data = context.get_context_data(ct.parms_id());
        size_t poly_degree = context_data->parms().poly_modulus_degree();
        size_t coeff_count = context_data->parms().coeff_modulus().size();
        
        // Each coefficient is 8 bytes, ciphertext has 2 polynomials
        return ct.size() * poly_degree * coeff_count * 8;
    }

    Ciphertext find_maximum(const vector<Ciphertext>& encrypted_bids) {
        if (encrypted_bids.empty()) {
            throw invalid_argument("Empty input vector");
        }
        
        if (encrypted_bids.size() == 1) {
            return encrypted_bids[0];
        }
        
        vector<Ciphertext> current_level = encrypted_bids;
        
        while (current_level.size() > 1) {
            vector<Ciphertext> next_level;
            
            for (size_t i = 0; i + 1 < current_level.size(); i += 2) {
                Ciphertext max_of_pair = pairwise_max(current_level[i], current_level[i + 1]);
                next_level.push_back(max_of_pair);
            }
            
            if (current_level.size() % 2 == 1) {
                next_level.push_back(current_level.back());
            }
            
            current_level = move(next_level);
        }
        
        return current_level[0];
    }

    int get_noise_budget(const Ciphertext& ct) {
        return decryptor->invariant_noise_budget(ct);
    }
};

void benchmark_maxid_auction(int num_bidders, int bit_width) {
    try {
        MaxIdBFV maxid_solver(bit_width);
        
        // Generate random bids
        vector<uint64_t> original_bids(num_bidders);
        random_device rd;
        mt19937 gen(rd());
        uint64_t max_value = (1ULL << bit_width) - 1;
        uniform_int_distribution<uint64_t> dist(1, max_value);
        
        for (auto& bid : original_bids) {
            bid = dist(gen);
        }
        
        vector<uint64_t> sorted_bids = original_bids;
        sort(sorted_bids.rbegin(), sorted_bids.rend());
        
        // Encrypt bids and measure communication size
        auto encrypt_start = high_resolution_clock::now();
        vector<Ciphertext> encrypted_bids;
        size_t total_comm_size = 0;
        
        for (auto bid : original_bids) {
            Ciphertext encrypted_bid = maxid_solver.encrypt_bid(bid);
            encrypted_bids.push_back(encrypted_bid);
            total_comm_size += maxid_solver.get_ciphertext_size(encrypted_bid);
        }
        auto encrypt_end = high_resolution_clock::now();
        
        // Find maximum
        auto compute_start = high_resolution_clock::now();
        Ciphertext max_encrypted = maxid_solver.find_maximum(encrypted_bids);
        auto compute_end = high_resolution_clock::now();
        
        // Decrypt result
        uint64_t computed_max = maxid_solver.decrypt_bid(max_encrypted);
        
        // Calculate metrics
        auto encrypt_time = duration_cast<milliseconds>(encrypt_end - encrypt_start).count();
        auto compute_time = duration_cast<milliseconds>(compute_end - compute_start).count();
        auto total_time = encrypt_time + compute_time;
        
        double comm_size_mb = total_comm_size / (1024.0 * 1024.0);
        size_t avg_bid_size_kb = total_comm_size / (num_bidders * 1024);
        int noise_budget = maxid_solver.get_noise_budget(max_encrypted);
        
        bool correct = (computed_max == sorted_bids[0]);
        double error_pct = correct ? 0.0 : 
            abs((double)computed_max - sorted_bids[0]) / sorted_bids[0] * 100.0;
        
        // Output results in clean format
        cout << num_bidders << "," << bit_width << "," << max_value << ","
             << computed_max << "," << sorted_bids[0] << ","
             << (correct ? "PASS" : "FAIL") << "," << error_pct << ","
             << encrypt_time << "," << compute_time << "," << total_time << ","
             << comm_size_mb << "," << avg_bid_size_kb << "," << noise_budget << endl;
        
    } catch (const exception& e) {
        cout << num_bidders << "," << bit_width << ",ERROR," << e.what() << endl;
    }
}

int main(int argc, char* argv[]) {
    // If arguments provided, run single benchmark
    if (argc >= 3) {
        int num_bidders = atoi(argv[1]);
        int domain_size = atoi(argv[2]);
        
        // Convert domain size to bit width
        // domain_size = 2^bit_width, so bit_width = log2(domain_size)
        int bit_width = 0;
        int temp = domain_size;
        while (temp > 1) {
            bit_width++;
            temp >>= 1;
        }
        // Add 1 to ensure we can represent all values up to domain_size
        if ((1ULL << bit_width) < domain_size) {
            bit_width++;
        }
        
        // Print CSV header
        cout << "Bidders,BitWidth,MaxValue,ComputedMax,ActualMax,Status,ErrorPct,"
             << "EncryptTime(ms),ComputeTime(ms),TotalTime(ms),"
             << "CommSize(MB),AvgBidSize(KB),NoiseBudget" << endl;
        
        benchmark_maxid_auction(num_bidders, bit_width);
        return 0;
    }
    
    // Otherwise, run all test configurations
    // Print CSV header
    cout << "Bidders,BitWidth,MaxValue,ComputedMax,ActualMax,Status,ErrorPct,"
         << "EncryptTime(ms),ComputeTime(ms),TotalTime(ms),"
         << "CommSize(MB),AvgBidSize(KB),NoiseBudget" << endl;
    
    // Test configurations based on your requirements
    vector<pair<int, int>> test_configs = {
        {100, 13},  // ~10000 max value
        {100, 10},  // ~1000 max value  
        {50, 10},   // ~1000 max value
        {25, 10},   // ~1000 max value
        {100, 7},   // ~100 max value
    };
    
    for (const auto& config : test_configs) {
        benchmark_maxid_auction(config.first, config.second);
    }
    
    return 0;
}
