cmake_minimum_required(VERSION 3.14)

project(test_ockam)

include_directories(../include)
add_subdirectory(.. _build)

add_executable(test_ockam test_ockam.c)

target_link_libraries(test_ockam ockam_atecc_vault)
install(TARGETS test_ockam DESTINATION bin)
