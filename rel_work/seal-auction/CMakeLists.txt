cmake_minimum_required(VERSION 3.13)

# The toolchain file must be set BEFORE the project() call
set(CMAKE_TOOLCHAIN_FILE "/home/architb/vcpkg/scripts/buildsystems/vcpkg.cmake" CACHE STRING "")

project(SEALAuction VERSION 1.0.0 LANGUAGES CXX)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/bin)

# Find the SEAL package
find_package(SEAL CONFIG REQUIRED)

# Add your executable
add_executable(auction_benchmark auction_benchmark.cpp)

# Link against SEAL
target_link_libraries(auction_benchmark PRIVATE SEAL::seal)
