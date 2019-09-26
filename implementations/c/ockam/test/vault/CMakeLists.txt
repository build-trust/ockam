cmake_minimum_required(VERSION 3.14)

project(test_atecc608a)

include_directories(../include)
add_subdirectory(.. _build)

add_executable(test_atecc608a test_atecc608a.c)

target_link_libraries(test_atecc608a ockam_vault)
install(TARGETS test_atecc608a DESTINATION bin)
