#include "addax-lib.h"
#include "net.hpp"
#include <thread>

// Global communication tracking variables
std::atomic<size_t> total_bytes_sent(0);
std::atomic<size_t> total_bytes_received(0);
std::atomic<size_t> hash_bytes_sent(0);
std::atomic<size_t> hash_bytes_received(0);
std::atomic<size_t> data_bytes_sent(0);
std::atomic<size_t> data_bytes_received(0);

// Wrapper functions to track communication
void sendShare_tracked(int fd, string& data) {
    sendShare(fd, data);
    total_bytes_sent += data.size();
    data_bytes_sent += data.size();
}

void recvShare_tracked(int fd, string& data) {
    recvShare(fd, data);
    total_bytes_received += data.size();
    data_bytes_received += data.size();
}

void sendHash_tracked(int fd, string& hash) {
    sendShare(fd, hash);
    total_bytes_sent += hash.size();
    hash_bytes_sent += hash.size();
}

void recvHash_tracked(int fd, string& hash) {
    recvShare(fd, hash);
    total_bytes_received += hash.size();
    hash_bytes_received += hash.size();
}

void load_all_shares(string dir_name, string s1_file_names,
                     string s2_file_names, vector<string>& s1_vec,
                     vector<string>& s2_vec) {
    int ad_num = s1_vec.size();
    ifstream in1(s1_file_names);
    ifstream in2(s2_file_names);
    for (int i = 0; i < ad_num; i++) {
        string n1, n2;
        getline(in1, n1);
        getline(in2, n2);
        ifstream s1file;
        s1file.open(dir_name + "/" + n1);
        std::stringstream ss1;
        ss1 << s1file.rdbuf();
        s1_vec[i] = ss1.str();
        ifstream s2file;
        s2file.open(dir_name + "/" + n2);
        std::stringstream ss2;
        ss2 << s2file.rdbuf();
        s2_vec[i] = ss2.str();
    }
    in1.close();
    in2.close();
}

void connect_to_publisher(int* fd, string ip) { 
    *fd = connect_to_addr(ip); 
}

void print_communication_stats(const string& phase) {
    cout << "\n=== Communication Stats for " << phase << " ===" << endl;
    cout << "Total bytes sent: " << total_bytes_sent << " (" 
         << (total_bytes_sent / 1024.0) << " KB, " 
         << (total_bytes_sent / (1024.0 * 1024.0)) << " MB)" << endl;
    cout << "Total bytes received: " << total_bytes_received << " (" 
         << (total_bytes_received / 1024.0) << " KB, " 
         << (total_bytes_received / (1024.0 * 1024.0)) << " MB)" << endl;
    cout << "Total communication: " << (total_bytes_sent + total_bytes_received) << " bytes (" 
         << ((total_bytes_sent + total_bytes_received) / 1024.0) << " KB, " 
         << ((total_bytes_sent + total_bytes_received) / (1024.0 * 1024.0)) << " MB)" << endl;
    cout << "Hash bytes sent: " << hash_bytes_sent << " bytes" << endl;
    cout << "Hash bytes received: " << hash_bytes_received << " bytes" << endl;
    cout << "Data bytes sent: " << data_bytes_sent << " bytes" << endl;
    cout << "Data bytes received: " << data_bytes_received << " bytes" << endl;
}

int main(int argc, char* argv[]) {
    Crypto env = Crypto();
    
    int ad_num = 100;
    bool parallel = true;
    int parallel_num_committee = 8;
    int parallel_num_sum = 8;
    bool parallel_sum = true;
    string s1_filenames = "";
    string s2_filenames = "";
    string dir_name = "";
    
    bool is_server = false;
    string publisher_ip = "127.0.0.1";
    int p_port = 6666;
    
    int opt;
    while ((opt = getopt(argc, argv, "a:b:s:S:d:i:p:k")) != -1) {
        if (opt == 'a') {
            ad_num = atoi(optarg);
        } else if (opt == 'b') {
            BUCKET_NUM = atoi(optarg);
        } else if (opt == 's') {
            s1_filenames = string(optarg);
        } else if (opt == 'S') {
            s2_filenames = string(optarg);
        } else if (opt == 'd') {
            dir_name = string(optarg);
        } else if (opt == 'i') {
            publisher_ip = string(optarg);
        } else if (opt == 'p') {
            p_port = atoi(optarg);
        } else if (opt == 'k') {
            is_server = true;
        }
    }

    cout << "=== Addax Non-Interactive with Network Communication ===" << endl;
    cout << "Mode: " << (is_server ? "SERVER" : "PUBLISHER") << endl;
    cout << "Advertisers: " << ad_num << ", Buckets: " << BUCKET_NUM << endl;

    vector<string> all_advs_s1_vec(ad_num);
    vector<string> all_advs_s2_vec(ad_num);
    
    load_all_shares(dir_name, s1_filenames, s2_filenames, all_advs_s1_vec, all_advs_s2_vec);

    double net_total = 0.0;
    double deserialize_total = 0.0;
    double compute_total = 0.0;
    system_clock::time_point starttime, endtime;
    system_clock::time_point start_total, end_total;
    start_total = system_clock::now();

    if (is_server) {
        // SERVER SIDE
        starttime = system_clock::now();
        
        // Step 1: Compute local sum
        Committee c2(BUCKET_NUM, LAMBDA, parallel, parallel_num_committee);
        c2.initShares(ad_num);
        c2.deserial_addShares_parallel_opt(all_advs_s2_vec);
        
        vector<vector<BIGNUM*>> c2_share = c2.getShares_opt();
        vector<BIGNUM*> sum_s2 = sumBNVec_opt(c2_share, env, parallel_sum, parallel_num_sum);
        
        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Step 2: Network communication - receive sum_s1 from publisher
        starttime = system_clock::now();
        
        int publisher_fd_write, publisher_fd_read;
        thread t1_connect(&connect_to_publisher, &publisher_fd_read,
                          publisher_ip + ":" + to_string(p_port));
        thread t2_connect(&connect_to_publisher, &publisher_fd_write,
                          publisher_ip + ":" + to_string(p_port + 1));
        
        t1_connect.join();
        t2_connect.join();
        assert(publisher_fd_write > 0);
        assert(publisher_fd_read > 0);
        
        // Reset communication counters for sum exchange
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;
        
        // Exchange sums with tracking
        string sum_s2_str = serializeShareVec_opt(sum_s2);
        string sum_s2_str_hash = sha256(sum_s2_str);
        
        cout << "Sum S2 serialized size: " << sum_s2_str.size() << " bytes" << endl;
        
        thread t1_hash(&sendHash_tracked, publisher_fd_write, ref(sum_s2_str_hash));
        string sum_s1_str_hash;
        thread t2_hash(&recvHash_tracked, publisher_fd_read, ref(sum_s1_str_hash));
        t2_hash.join();
        t1_hash.join();

        thread t1(&sendShare_tracked, publisher_fd_write, ref(sum_s2_str));
        string sum_s1_str;
        thread t2(&recvShare_tracked, publisher_fd_read, ref(sum_s1_str));
        t2.join();
        t1.join();
        
        cout << "Sum S1 received size: " << sum_s1_str.size() << " bytes" << endl;
        print_communication_stats("Sum Exchange");
        
        endtime = system_clock::now();
        net_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Step 3: Deserialize received sum_s1
        starttime = system_clock::now();
        assert(sha256(sum_s1_str) == sum_s1_str_hash);
        vector<BIGNUM*> sum_s1 = c2.bn_deserializeShare_opt(sum_s1_str);
        endtime = system_clock::now();
        deserialize_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Step 4: Compute max bid
        starttime = system_clock::now();
        vector<vector<BIGNUM*>> share_input;
        share_input.emplace_back(sum_s1);
        share_input.emplace_back(sum_s2);
        vector<BIGNUM*> sum_vec_s = sumBNVec_opt(share_input, env);
        int decode_bid = decode_bit_vec_opt(sum_vec_s);
        
        cout << "Max bid value: " << decode_bid << endl;
        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Step 5: Find winner - Reset counters for winner finding
        starttime = system_clock::now();
        
        // Reset communication counters for winner finding
        size_t sum_exchange_total = total_bytes_sent + total_bytes_received;
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;

        // Prepare sequence
        vector<int> ids;
        for (int i = 0; i < ad_num; i++) {
            ids.push_back(i);
        }
        srand(100);

        vector<int> seq_ids;
        while (!ids.empty()) {
            int tmp_id = rand() % ids.size();
            seq_ids.push_back(ids[tmp_id]);
            ids.erase(ids.begin() + tmp_id);
        }

        // Serialize ALL candidates' bit shares at once
        string all_bits_s2_str;
        for (int i = 0; i < seq_ids.size(); i++) {
            vector<BIGNUM*> s2_bits;
            for (int j = 0; j < LAMBDA; j++) {
                s2_bits.push_back(c2.revealBitShare_opt(seq_ids[i], decode_bid * LAMBDA + j));
            }
            string s2_bits_str = serializeBit(s2_bits);
            all_bits_s2_str.append(formatMsg(s2_bits_str));
        }

        cout << "All bits S2 serialized size: " << all_bits_s2_str.size() << " bytes" << endl;

        // SINGLE network exchange for ALL candidates
        string all_bits_s2_hash = sha256(all_bits_s2_str);

        thread t1_winner_hash(&sendHash_tracked, publisher_fd_write, ref(all_bits_s2_hash));
        string all_bits_s1_hash;
        thread t2_winner_hash(&recvHash_tracked, publisher_fd_read, ref(all_bits_s1_hash));
        t2_winner_hash.join();
        t1_winner_hash.join();

        thread t1_winner(&sendShare_tracked, publisher_fd_write, ref(all_bits_s2_str));
        string all_bits_s1_str;
        thread t2_winner(&recvShare_tracked, publisher_fd_read, ref(all_bits_s1_str));
        t2_winner.join();
        t1_winner.join();

        cout << "All bits S1 received size: " << all_bits_s1_str.size() << " bytes" << endl;
        print_communication_stats("Winner Finding");

        assert(sha256(all_bits_s1_str) == all_bits_s1_hash);

        // LOCAL processing to find winner
        int winner_id = -1;
        for (int i = 0; i < seq_ids.size(); i++) {
            // Extract this candidate's s1 bits from the batch
            uint32_t size_s1_str = ntohl(*((uint32_t*)all_bits_s1_str.substr(0, sizeof(uint32_t)).c_str()));
            string s1_bits_str = all_bits_s1_str.substr(sizeof(uint32_t), size_s1_str);
            all_bits_s1_str = all_bits_s1_str.substr(sizeof(uint32_t) + size_s1_str);
            
            vector<BIGNUM*> s1_bits = deserializeBit(s1_bits_str);
            
            // Get this candidate's s2 bits
            vector<BIGNUM*> s2_bits;
            for (int j = 0; j < LAMBDA; j++) {
                s2_bits.push_back(c2.revealBitShare_opt(seq_ids[i], decode_bid * LAMBDA + j));
            }
            
            // Combine and check
            vector<BIGNUM*> s;
            for (int j = 0; j < LAMBDA; j++) {
                BIGNUM* v = BN_new();
                env.add_mod(v, s1_bits[j], s2_bits[j]);
                s.push_back(v);
            }
            
            int bit = decode_bit(s);
            if (bit == 1) {
                winner_id = seq_ids[i];
                cout << "Winner: " << winner_id << endl;
                break;
            }
            
            // Cleanup
            for (auto& bn : s) BN_free(bn);
        }

        endtime = system_clock::now();

        // Step 6: Find second price - Reset counters for second price
        starttime = system_clock::now();
        
        size_t winner_finding_total = total_bytes_sent + total_bytes_received;
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;
        
        // Get removed shares from server
        vector<BIGNUM*> removed_s2 = c2.revealAdShare_opt(winner_id);
        
        // Send removed_s2 and receive removed_s1 from publisher
        string removed_s2_str = serializeShareVec_opt(removed_s2);
        string removed_s2_hash = sha256(removed_s2_str);
        
        cout << "Removed S2 serialized size: " << removed_s2_str.size() << " bytes" << endl;
        
        thread t1_rem_hash(&sendHash_tracked, publisher_fd_write, ref(removed_s2_hash));
        string removed_s1_hash;
        thread t2_rem_hash(&recvHash_tracked, publisher_fd_read, ref(removed_s1_hash));
        t2_rem_hash.join();
        t1_rem_hash.join();

        thread t1_rem(&sendShare_tracked, publisher_fd_write, ref(removed_s2_str));
        string removed_s1_str;
        thread t2_rem(&recvShare_tracked, publisher_fd_read, ref(removed_s1_str));
        t2_rem.join();
        t1_rem.join();
        
        cout << "Removed S1 received size: " << removed_s1_str.size() << " bytes" << endl;
        print_communication_stats("Second Price Calculation");
        
        assert(sha256(removed_s1_str) == removed_s1_hash);
        vector<BIGNUM*> removed_s1 = c2.bn_deserializeShare_opt(removed_s1_str);
        
        // Remove winner's shares
        subShare_opt(sum_s1, removed_s1, env);
        subShare_opt(sum_s2, removed_s2, env);
        vector<vector<BIGNUM*>> removed_share_input;
        removed_share_input.push_back(sum_s1);
        removed_share_input.push_back(sum_s2);
        vector<BIGNUM*> sum_vec_removed = sumBNVec_opt(removed_share_input, env);
        int second_price = decode_bit_vec_opt(sum_vec_removed);
        
        cout << "Second price: " << second_price << endl;
        
        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Print total communication summary
        size_t second_price_total = total_bytes_sent + total_bytes_received;
        size_t grand_total = sum_exchange_total + winner_finding_total + second_price_total;
        
        cout << "\n=== TOTAL COMMUNICATION BREAKDOWN ===" << endl;
        cout << "Sum Exchange: " << sum_exchange_total << " bytes (" 
             << (sum_exchange_total / 1024.0) << " KB)" << endl;
        cout << "Winner Finding: " << winner_finding_total << " bytes (" 
             << (winner_finding_total / 1024.0) << " KB)" << endl;
        cout << "Second Price: " << second_price_total << " bytes (" 
             << (second_price_total / 1024.0) << " KB)" << endl;
        cout << "GRAND TOTAL: " << grand_total << " bytes (" 
             << (grand_total / 1024.0) << " KB, " 
             << (grand_total / (1024.0 * 1024.0)) << " MB)" << endl;

        // Cleanup
        close(publisher_fd_read);
        close(publisher_fd_write);
        freeSumBNvec_opt(sum_s1);
        freeSumBNvec_opt(sum_s2);
        freeSumBNvec_opt(sum_vec_s);
        freeSumBNvec_opt(sum_vec_removed);
        c2.free();
        
    } else {
        // PUBLISHER SIDE
        starttime = system_clock::now();

        // Step 1: Compute local sum
        Committee c1(BUCKET_NUM, LAMBDA, parallel, parallel_num_committee);
        c1.initShares(ad_num);
        c1.deserial_addShares_parallel_opt(all_advs_s1_vec);

        vector<vector<BIGNUM*>> c1_share = c1.getShares_opt();
        vector<BIGNUM*> sum_s1 = sumBNVec_opt(c1_share, env, parallel_sum, parallel_num_sum);

        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();
        cout << "TIME: deserialize share vecs per committee: " 
            << duration_cast<std::chrono::duration<double>>(endtime - starttime).count() << endl;

        // Step 2: Setup server connections
        starttime = system_clock::now();

        // Setup two server sockets
        int listen_fd_send = socket(AF_INET, SOCK_STREAM, 0);
        int listen_fd_recv = socket(AF_INET, SOCK_STREAM, 0);

        int reuse = 1;
        setsockopt(listen_fd_send, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));
        setsockopt(listen_fd_recv, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));

        struct sockaddr_in servaddr_send, servaddr_recv;
        memset(&servaddr_send, 0, sizeof(servaddr_send));
        memset(&servaddr_recv, 0, sizeof(servaddr_recv));

        servaddr_send.sin_family = AF_INET;
        servaddr_send.sin_addr.s_addr = INADDR_ANY;
        servaddr_send.sin_port = htons(p_port);

        servaddr_recv.sin_family = AF_INET;
        servaddr_recv.sin_addr.s_addr = INADDR_ANY;
        servaddr_recv.sin_port = htons(p_port + 1);

        assert(bind(listen_fd_send, (sockaddr*)&servaddr_send, sizeof(servaddr_send)) >= 0);
        assert(bind(listen_fd_recv, (sockaddr*)&servaddr_recv, sizeof(servaddr_recv)) >= 0);
        assert(listen(listen_fd_send, 5) >= 0);
        assert(listen(listen_fd_recv, 5) >= 0);

        cout << "Publisher listening on ports " << p_port << " and " << (p_port + 1) << endl;

        // Accept connections
        struct sockaddr_in cliaddr;
        socklen_t clilen = sizeof(cliaddr);
        int server_fd_send = accept(listen_fd_send, (sockaddr*)&cliaddr, &clilen);
        int server_fd_recv = accept(listen_fd_recv, (sockaddr*)&cliaddr, &clilen);

        assert(server_fd_send > 0);
        assert(server_fd_recv > 0);
        cout << "Server connected!" << endl;

        endtime = system_clock::now();
        net_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();

        // Step 3: Exchange sums with communication tracking
        starttime = system_clock::now();
        
        // Reset communication counters for sum exchange
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;

        string sum_s1_str = serializeShareVec_opt(sum_s1);
        string sum_s1_str_hash = sha256(sum_s1_str);
        
        cout << "Sum S1 serialized size: " << sum_s1_str.size() << " bytes" << endl;

        // Publisher receives server's hash, then sends its own hash
        string sum_s2_str_hash;
        thread t1_hash(&recvHash_tracked, server_fd_recv, ref(sum_s2_str_hash));
        thread t2_hash(&sendHash_tracked, server_fd_send, ref(sum_s1_str_hash));
        t1_hash.join();
        t2_hash.join();

        // Publisher receives server's data, then sends its own data
        string sum_s2_str;
        thread t1(&recvShare_tracked, server_fd_recv, ref(sum_s2_str));
        thread t2(&sendShare_tracked, server_fd_send, ref(sum_s1_str));
        t1.join();
        t2.join();
        
        cout << "Sum S2 received size: " << sum_s2_str.size() << " bytes" << endl;
        print_communication_stats("Sum Exchange");

        endtime = system_clock::now();
        net_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();
        cout << "TIME: send + recv shares " 
            << duration_cast<std::chrono::duration<double>>(endtime - starttime).count() << endl;

        // Step 4: Decode max bid and prepare for winner finding
        starttime = system_clock::now();
        
        assert(sha256(sum_s2_str) == sum_s2_str_hash);
        vector<BIGNUM*> sum_s2 = c1.bn_deserializeShare_opt(sum_s2_str);

        vector<vector<BIGNUM*>> share_input;
        share_input.emplace_back(sum_s1);
        share_input.emplace_back(sum_s2);
        vector<BIGNUM*> sum_vec_s = sumBNVec_opt(share_input, env);
        int decode_bid = decode_bit_vec_opt(sum_vec_s);
        
        cout << "Max bid value: " << decode_bid << endl;

        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();
        cout << "TIME: decode max: " 
            << duration_cast<std::chrono::duration<double>>(endtime - starttime).count() << endl;

        // Step 5: Winner finding with communication tracking
        starttime = system_clock::now();
        
        // Reset communication counters for winner finding
        size_t sum_exchange_total = total_bytes_sent + total_bytes_received;
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;

        // Prepare sequence (same seed as server)
        vector<int> ids;
        for (int i = 0; i < ad_num; i++) {
            ids.push_back(i);
        }
        srand(100); // Same seed as server

        vector<int> seq_ids;
        while (!ids.empty()) {
            int tmp_id = rand() % ids.size();
            seq_ids.push_back(ids[tmp_id]);
            ids.erase(ids.begin() + tmp_id);
        }

        // Serialize ALL candidates' bit shares at once
        string all_bits_s1_str;
        for (int i = 0; i < seq_ids.size(); i++) {
            vector<BIGNUM*> s1_bits;
            for (int j = 0; j < LAMBDA; j++) {
                s1_bits.push_back(c1.revealBitShare_opt(seq_ids[i], decode_bid * LAMBDA + j));
            }
            string s1_bits_str = serializeBit(s1_bits);
            all_bits_s1_str.append(formatMsg(s1_bits_str));
        }

        cout << "All bits S1 serialized size: " << all_bits_s1_str.size() << " bytes" << endl;

        // SINGLE network exchange for ALL candidates
        string all_bits_s1_hash = sha256(all_bits_s1_str);

        // Publisher receives server's batch hash, then sends its own batch hash
        string all_bits_s2_hash;
        thread t1_winner_hash(&recvHash_tracked, server_fd_recv, ref(all_bits_s2_hash));
        thread t2_winner_hash(&sendHash_tracked, server_fd_send, ref(all_bits_s1_hash));
        t1_winner_hash.join();
        t2_winner_hash.join();

        // Publisher receives server's batch data, then sends its own batch data
        string all_bits_s2_str;
        thread t1_winner(&recvShare_tracked, server_fd_recv, ref(all_bits_s2_str));
        thread t2_winner(&sendShare_tracked, server_fd_send, ref(all_bits_s1_str));
        t1_winner.join();
        t2_winner.join();

        cout << "All bits S2 received size: " << all_bits_s2_str.size() << " bytes" << endl;
        print_communication_stats("Winner Finding");

        assert(sha256(all_bits_s2_str) == all_bits_s2_hash);

        // LOCAL processing to find winner
        int winner_id = -1;
        for (int i = 0; i < seq_ids.size(); i++) {
            // Extract this candidate's s2 bits from the server's batch
            uint32_t size_s2_str = ntohl(*((uint32_t*)all_bits_s2_str.substr(0, sizeof(uint32_t)).c_str()));
            string s2_bits_str = all_bits_s2_str.substr(sizeof(uint32_t), size_s2_str);
            all_bits_s2_str = all_bits_s2_str.substr(sizeof(uint32_t) + size_s2_str);

            vector<BIGNUM*> s2_bits = deserializeBit(s2_bits_str);

            // Get this candidate's s1 bits (we have them locally)
            vector<BIGNUM*> s1_bits;
            for (int j = 0; j < LAMBDA; j++) {
                s1_bits.push_back(c1.revealBitShare_opt(seq_ids[i], decode_bid * LAMBDA + j));
            }

            // Combine and check
            vector<BIGNUM*> s;
            for (int j = 0; j < LAMBDA; j++) {
                BIGNUM* v = BN_new();
                env.add_mod(v, s1_bits[j], s2_bits[j]);
                s.push_back(v);
            }

            int bit = decode_bit(s);
            if (bit == 1) {
                winner_id = seq_ids[i];
                cout << "Publisher: Winner found: " << winner_id << endl;
                break;
            }

            // Cleanup
            for (auto& bn : s) BN_free(bn);
        }

        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();
        cout << "TIME: finding index of winner: " 
            << duration_cast<std::chrono::duration<double>>(endtime - starttime).count() << endl;

        // Step 6: Second price calculation with communication tracking
        starttime = system_clock::now();
        
        // Reset communication counters for second price
        size_t winner_finding_total = total_bytes_sent + total_bytes_received;
        total_bytes_sent = 0;
        total_bytes_received = 0;
        hash_bytes_sent = 0;
        hash_bytes_received = 0;
        data_bytes_sent = 0;
        data_bytes_received = 0;

        // Send removed shares for second price
        vector<BIGNUM*> removed_s1 = c1.revealAdShare_opt(winner_id);
        string removed_s1_str = serializeShareVec_opt(removed_s1);
        string removed_s1_hash = sha256(removed_s1_str);
        
        cout << "Removed S1 serialized size: " << removed_s1_str.size() << " bytes" << endl;

        // Publisher receives server's removed hash, then sends its own removed hash
        string removed_s2_hash;
        thread t1_rem_hash(&recvHash_tracked, server_fd_recv, ref(removed_s2_hash));
        thread t2_rem_hash(&sendHash_tracked, server_fd_send, ref(removed_s1_hash));
        t1_rem_hash.join();
        t2_rem_hash.join();

        // Publisher receives server's removed data, then sends its own removed data
        string removed_s2_str;
        thread t1_rem(&recvShare_tracked, server_fd_recv, ref(removed_s2_str));
        thread t2_rem(&sendShare_tracked, server_fd_send, ref(removed_s1_str));
        t1_rem.join();
        t2_rem.join();
        
        cout << "Removed S2 received size: " << removed_s2_str.size() << " bytes" << endl;
        print_communication_stats("Second Price Calculation");

        endtime = system_clock::now();
        compute_total += duration_cast<std::chrono::duration<double>>(endtime - starttime).count();
        cout << "TIME: finding second highest price: " 
            << duration_cast<std::chrono::duration<double>>(endtime - starttime).count() << endl;

        // Print total communication summary for publisher
        size_t second_price_total = total_bytes_sent + total_bytes_received;
        size_t grand_total = sum_exchange_total + winner_finding_total + second_price_total;
        
        cout << "\n=== PUBLISHER TOTAL COMMUNICATION BREAKDOWN ===" << endl;
        cout << "Sum Exchange: " << sum_exchange_total << " bytes (" 
            << (sum_exchange_total / 1024.0) << " KB)" << endl;
        cout << "Winner Finding: " << winner_finding_total << " bytes (" 
            << (winner_finding_total / 1024.0) << " KB)" << endl;
        cout << "Second Price: " << second_price_total << " bytes (" 
            << (second_price_total / 1024.0) << " KB)" << endl;
        cout << "GRAND TOTAL: " << grand_total << " bytes (" 
            << (grand_total / 1024.0) << " KB, " 
            << (grand_total / (1024.0 * 1024.0)) << " MB)" << endl;

        cout << "Publisher completed auction participation" << endl;

        // Close connections
        close(server_fd_send);
        close(server_fd_recv);
        close(listen_fd_send);
        close(listen_fd_recv);

        // Cleanup
        freeSumBNvec_opt(sum_s1);
        freeSumBNvec_opt(sum_s2);
        freeSumBNvec_opt(sum_vec_s);
        c1.free();
    }

    
    end_total = system_clock::now();
    double total_time = duration_cast<std::chrono::duration<double>>(end_total - start_total).count();
    
    cout << "\n=== Final Timing Results ===" << endl;
    cout << "TIME: network total: " << net_total << endl;
    cout << "TIME: serialize + deserialize total: " << deserialize_total << endl;
    cout << "TIME: compute total: " << compute_total << endl;
    cout << "TIME: total: " << total_time << endl;
    
    return 0;
}
