cmake_minimum_required(VERSION 3.14)
project(examples)

add_subdirectory(vault/aead_aes_128_gcm)
add_subdirectory(vault/initialize/default)
add_subdirectory(vault/random)
add_subdirectory(vault/sha256)
