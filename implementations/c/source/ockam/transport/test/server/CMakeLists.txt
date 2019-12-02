cmake_minimum_required(VERSION 3.10)

###########################
# Set directory locations #
###########################

set(SERVER_TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)
set(CMAKE_C_FLAGS "-std=c99 -g")

# set the project name
project(test_server)

# Get the headers
include_directories(
  ../../inc
  ../inc
  ../
  )

# Add the source files
file(GLOB SOURCES "./*.c" "../../src/*.c" "../*.c" "../../inc/*.h" "../*.h")

add_executable(test_server ${SOURCES})
