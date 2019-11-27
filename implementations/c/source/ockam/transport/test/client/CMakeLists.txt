cmake_minimum_required(VERSION 3.10)

set(CMAKE_C_FLAGS "-std=c99")

# set the project name
project(test_client)

# Get the headers
include_directories(
  ../../inc
  ../inc
  ../
  )

# Add the source files
file(GLOB SOURCES "*.c" "../../src/*.c" "../*.c")

add_executable(test_client ${SOURCES})
